//! Backend de Windows (Win32).
//!
//! Loop orientado a eventos: `GetMessageW` bloquea el hilo hasta que llega un
//! mensaje, así que en reposo el consumo de CPU es 0%. La isla es una ventana
//! con transparencia por píxel (`WS_EX_LAYERED`): la imagen se entrega a Windows
//! con `UpdateLayeredWindow` y el sistema la mantiene en pantalla sin que Atoll
//! haga nada más.
//!
//! Timers permitidos (solo cuatro):
//! 1. **Animación (`ANIMATION_TIMER`):** solo existe mientras la isla se está
//!    moviendo (`animation_ms`) y se apaga apenas termina. Con `animation_ms = 0`
//!    ni siquiera se arma.
//! 2. **Reloj (`CLOCK_TIMER`):** un solo disparo por minuto, alineado al próximo
//!    cambio de minuto (`SetCoalescableTimer` sin coalescencia). Se re-sincroniza
//!    ante `WM_TIMECHANGE` y al volver de suspensión (`WM_POWERBROADCAST`).
//! 3. **Debounce de configuración (`RELOAD_TIMER`):** un solo disparo de 50 ms
//!    cuando el hilo observador avisa (`WM_APP + 1`) que cambió `config.ini`.
//! 4. **Retardo de hover (`HOVER_DELAY_TIMER`):** un solo disparo de hasta 2000 ms
//!    al entrar el mouse cuando `hover_delay_ms > 0`; se cancela (`KillTimer`) si
//!    el mouse sale antes de que venza. Con `hover_delay_ms = 0` no se arma.
//!
//! Recarga en vivo: un hilo auxiliar con pila reducida (64 KB) queda bloqueado en
//! `ReadDirectoryChangesW` sobre la carpeta de configuración (cero polling, cero
//! CPU en reposo) y envía `PostMessageW(hwnd, WM_APP + 1, 0, 0)` cuando se toca
//! `config.ini` (incluyendo guardado por archivo temporal + renombrado).

use std::cell::Cell;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::{copy_nonoverlapping, null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use atoll_render::{Frame, Size};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HWND, INVALID_HANDLE_VALUE, LPARAM, LRESULT, POINT, SIZE, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_CREATION, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, KillTimer, LoadCursorW, PostMessageW, PostQuitMessage, RegisterClassExW,
    SetCoalescableTimer, SetProcessDPIAware, SetTimer, ShowWindow, TranslateMessage,
    UnregisterClassW, UpdateLayeredWindow, IDC_ARROW, MSG, PBT_APMRESUMEAUTOMATIC,
    PBT_APMRESUMESUSPEND, SM_CXSCREEN, SW_SHOWNOACTIVATE, TIMERV_NO_COALESCING, ULW_ALPHA, WM_APP,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_MOUSEMOVE, WM_POWERBROADCAST, WM_RBUTTONUP,
    WM_TIMECHANGE, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::animation::{Animator, HoverAction};
use crate::clock::{local_time, LocalTime};
use crate::dpi::{scale_for_dpi, scale_px, scale_size};
use crate::{Backend, IslandConfig, Painter, PlatformError};

/// Nombre de la clase de ventana registrada en Win32.
const CLASS_NAME: &str = "AtollIsland";

/// Nombre del archivo de configuración observado dentro de la carpeta.
const CONFIG_FILE_NAME: &str = "config.ini";

/// Mensaje que Windows envía cuando el mouse sale de la ventana (0x02A3).
///
/// Se define acá porque `windows-sys` lo ubica en el módulo `Controls`, y
/// activar esa feature solo por una constante agrega código que no usamos.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// Mensaje enviado por el hilo observador cuando cambia `config.ini`.
const WM_CONFIG_CHANGED: u32 = WM_APP + 1;

/// Identificador del timer de la animación.
const ANIMATION_TIMER: usize = 1;

/// Identificador del timer del reloj (un solo disparo por minuto).
const CLOCK_TIMER: usize = 2;

/// Identificador del timer de debounce para recargar la configuración (un solo disparo).
const RELOAD_TIMER: usize = 3;

/// Identificador del timer de retardo antes de expandir por hover (un solo disparo).
const HOVER_DELAY_TIMER: usize = 4;

/// Margen que se suma a la espera del reloj, en milisegundos.
///
/// Evita que el timer salte unos milisegundos antes del cambio de minuto y haya
/// que armarlo dos veces.
const CLOCK_MARGIN_MS: u32 = 20;

/// Espera de respaldo, en milisegundos, si no se pudiera leer la hora.
const CLOCK_FALLBACK_MS: u32 = 60_000;

/// Milisegundos de debounce antes de releer `config.ini` tras un aviso del SO.
const RELOAD_DEBOUNCE_MS: u32 = 50;

/// Tamaño de pila del hilo observador de configuración (64 KB).
const WATCHER_STACK_BYTES: usize = 64 * 1024;

/// Milisegundos entre cuadros de la animación (≈ 60 cuadros por segundo).
const FRAME_INTERVAL_MS: u32 = 16;

/// Bandera para que el hilo observador deje de publicar mensajes al cerrarse la ventana.
static WINDOW_CLOSED: AtomicBool = AtomicBool::new(false);

/// Medidas lógicas y parámetros de comportamiento (`Copy`) de la isla, guardados en `thread_local!`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LogicalMetrics {
    size: Size,
    hover_size: Size,
    top_margin: u32,
    hover_delay_ms: u32,
    animation_ms: u32,
}

impl From<&IslandConfig> for LogicalMetrics {
    fn from(config: &IslandConfig) -> Self {
        Self {
            size: config.size,
            hover_size: config.hover_size,
            top_margin: config.top_margin,
            hover_delay_ms: config.hover_delay_ms,
            animation_ms: config.animation_ms,
        }
    }
}

thread_local! {
    /// Estado de la animación. El loop de mensajes corre en un solo hilo, así
    /// que alcanza con un `Cell` (y evita pánicos por reentrada del `RefCell`).
    static ANIMATOR: Cell<Option<Animator>> = const { Cell::new(None) };
    /// Margen superior de la isla, en píxeles físicos (ya escalado; `0` en modo `notch`).
    static TOP_MARGIN: Cell<i32> = const { Cell::new(0) };
    /// Medidas lógicas actuales de la isla, para recalcular tamaños cuando cambia el DPI o la config.
    static CONFIG: Cell<Option<LogicalMetrics>> = const { Cell::new(None) };
    /// Factor de escala actual del monitor (1,0 = 100 %).
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
    /// Quien dibuja el contenido de la isla. Se saca del `Cell` mientras dibuja
    /// y se vuelve a guardar, para no depender de préstamos entre mensajes.
    static PAINTER: Cell<Option<Box<dyn Painter>>> = const { Cell::new(None) };
    /// Hora con la que se dibujó el último cuadro: sirve para no redibujar si el texto no cambió.
    static LAST_DRAWN: Cell<Option<LocalTime>> = const { Cell::new(None) };
}

/// Backend nativo para Windows.
pub(crate) struct WindowsBackend;

impl Backend for WindowsBackend {
    fn run(
        &mut self,
        config: &IslandConfig,
        painter: Box<dyn Painter>,
    ) -> Result<(), PlatformError> {
        WINDOW_CLOSED.store(false, Ordering::SeqCst);
        PAINTER.set(Some(painter));
        // SAFETY: todas las llamadas Win32 ocurren en este hilo, que es el
        // dueño de la ventana y de su cola de mensajes.
        let result = unsafe { run_island(config) };
        WINDOW_CLOSED.store(true, Ordering::SeqCst);
        PAINTER.set(None);
        ANIMATOR.set(None);
        LAST_DRAWN.set(None);
        CONFIG.set(None);
        result
    }
}

/// Convierte un `&str` a UTF-16 terminado en cero, como pide Win32.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Convierte una ruta a UTF-16 terminado en cero, como pide Win32.
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Captura el último error del SO junto con el nombre de la llamada que falló.
fn os_error(call: &'static str) -> PlatformError {
    // SAFETY: `GetLastError` no tiene precondiciones.
    let code = unsafe { GetLastError() };
    PlatformError::Os { call, code }
}

/// Convierte píxeles sin signo a `i32` de Win32, saturando si no entran.
fn to_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// Calcula la esquina superior izquierda para una isla de ancho `width`,
/// centrada horizontalmente en la pantalla principal.
fn island_origin(width: u32) -> (i32, i32) {
    // SAFETY: sin precondiciones.
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let left = (screen_width - to_i32(width)).max(0) / 2;
    (left, TOP_MARGIN.get())
}

/// Abre la isla y corre el loop de mensajes hasta que se cierra.
///
/// # Safety
///
/// Debe llamarse desde un único hilo, que pasa a ser el dueño de la ventana.
unsafe fn run_island(config: &IslandConfig) -> Result<(), PlatformError> {
    let metrics = LogicalMetrics::from(config);
    CONFIG.set(Some(metrics));

    // SAFETY: sin precondiciones. Per-Monitor-V2: Windows no escala la ventana y
    // avisa con `WM_DPICHANGED` cuando cambia el DPI.
    if unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) } == 0 {
        // Windows más viejo (o ya configurado por manifiesto): se usa el DPI del sistema.
        // SAFETY: sin precondiciones.
        unsafe { SetProcessDPIAware() };
    }

    // SAFETY: con puntero nulo devuelve el módulo del ejecutable actual.
    let hinstance = unsafe { GetModuleHandleW(null()) };
    let class_name = wide(CLASS_NAME);

    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: null_mut(),
        // SAFETY: cursor estándar del sistema, no requiere liberar.
        hCursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW) },
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: null_mut(),
    };

    // SAFETY: `class` y `class_name` viven hasta después de la llamada.
    if unsafe { RegisterClassExW(&class) } == 0 {
        return Err(os_error("RegisterClassExW"));
    }

    // La ventana nace en el origen de la pantalla principal, todavía sin mostrarse: sirve
    // para preguntarle a Windows el DPI de ese monitor. Su tamaño y posición definitivos
    // los fija `UpdateLayeredWindow` al presentar el primer cuadro.
    // SAFETY: la clase ya está registrada y los punteros son válidos.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            to_i32(config.size.width),
            to_i32(config.size.height),
            null_mut(),
            null_mut(),
            hinstance,
            null(),
        )
    };
    if hwnd.is_null() {
        let err = os_error("CreateWindowExW");
        // SAFETY: la clase fue registrada arriba con este mismo nombre.
        unsafe { UnregisterClassW(class_name.as_ptr(), hinstance) };
        return Err(err);
    }

    // SAFETY: `hwnd` es válido y pertenece a este hilo.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    apply_scale(metrics, scale_for_dpi(dpi));

    // Primer cuadro, al tamaño físico que corresponde a este monitor.
    let first_size = scale_size(config.size, SCALE.get());
    let first = match paint(first_size) {
        Some((frame, time)) if frame.size() == first_size => Ok((frame, time)),
        _ => Err(PlatformError::InvalidFrame),
    };
    let (left, top) = island_origin(first_size.width);
    let presented = first.and_then(|(frame, time)| {
        // SAFETY: `hwnd` es válido y pertenece a este hilo.
        unsafe { present_frame(hwnd, left, top, &frame) }.map(|()| time)
    });
    let first_time = match presented {
        Ok(time) => time,
        Err(err) => {
            // SAFETY: `hwnd` es válido; la clase fue registrada arriba.
            unsafe {
                DestroyWindow(hwnd);
                UnregisterClassW(class_name.as_ptr(), hinstance);
            }
            return Err(err);
        }
    };

    LAST_DRAWN.set(Some(first_time));

    if let Some(dir) = config.config_dir.clone() {
        spawn_config_watcher(hwnd, dir);
    }

    // SAFETY: `hwnd` es válido.
    unsafe {
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        arm_clock(hwnd);
    }

    // SAFETY: `MSG` es un struct de datos planos; cero es un valor válido.
    let mut message: MSG = unsafe { zeroed() };
    let result = loop {
        // SAFETY: `message` es válido y el hilo es dueño de la cola.
        // Bloquea hasta que haya un mensaje: acá el CPU está en 0%.
        let status = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if status > 0 {
            // SAFETY: `message` fue completado por `GetMessageW`.
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        } else if status == 0 {
            break Ok(());
        } else {
            break Err(os_error("GetMessageW"));
        }
    };

    // SAFETY: la ventana ya fue destruida; se libera la clase registrada.
    unsafe { UnregisterClassW(class_name.as_ptr(), hinstance) };
    result
}

/// Inicia el hilo auxiliar que observa cambios sobre `config.ini` en `dir`.
///
/// Si el hilo no se puede crear, se informa una sola vez por `stderr` y la isla
/// sigue funcionando sin recarga en vivo.
fn spawn_config_watcher(hwnd: HWND, dir: PathBuf) {
    let hwnd_raw = hwnd as isize;
    let spawned = thread::Builder::new()
        .name("atoll-config-watch".to_owned())
        .stack_size(WATCHER_STACK_BYTES)
        .spawn(move || {
            watch_config_dir(hwnd_raw, &dir);
        });
    if let Err(err) = spawned {
        eprintln!("atoll: no se pudo crear el hilo de recarga en vivo: {err}");
    }
}

/// Bucle del hilo observador: bloqueado en `ReadDirectoryChangesW` (0% CPU en reposo).
fn watch_config_dir(hwnd_raw: isize, dir: &Path) {
    let wide_dir = wide_path(dir);
    // SAFETY: `wide_dir` es una cadena UTF-16 terminada en cero válida. Se abre el
    // directorio con `FILE_FLAG_BACKUP_SEMANTICS` y permisos compartidos completos.
    let handle = unsafe {
        CreateFileW(
            wide_dir.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        // SAFETY: `GetLastError` no tiene precondiciones.
        let code = unsafe { GetLastError() };
        eprintln!(
            "atoll: no se pudo observar la carpeta de configuración '{}' (código {code})",
            dir.display()
        );
        return;
    }

    // Buffer alineado a 4 bytes (como exige `FILE_NOTIFY_INFORMATION`).
    let mut buffer = [0u32; 1024];
    let buffer_byte_len = (buffer.len() * size_of::<u32>()) as u32;
    let filter = FILE_NOTIFY_CHANGE_FILE_NAME
        | FILE_NOTIFY_CHANGE_LAST_WRITE
        | FILE_NOTIFY_CHANGE_SIZE
        | FILE_NOTIFY_CHANGE_CREATION;

    loop {
        if WINDOW_CLOSED.load(Ordering::Relaxed) {
            break;
        }
        let mut bytes_returned: u32 = 0;
        // SAFETY: `handle` es un directorio válido y `buffer` está alineado a `u32`
        // con tamaño `buffer_byte_len`. La llamada es síncrona y bloquea el hilo.
        let ok = unsafe {
            ReadDirectoryChangesW(
                handle,
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer_byte_len,
                0,
                filter,
                &mut bytes_returned,
                null_mut(),
                None,
            )
        };
        if ok == 0 || WINDOW_CLOSED.load(Ordering::Relaxed) {
            break;
        }

        // SAFETY: `buffer` tiene `buffer.len() * 4` bytes inicializados.
        let raw_bytes = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr().cast::<u8>(),
                buffer.len() * size_of::<u32>(),
            )
        };
        let n = (bytes_returned as usize).min(raw_bytes.len());
        // Si `bytes_returned == 0` con `ok != 0`, el buffer se llenó: avisamos por las dudas.
        if n == 0 || mentions_config_file(&raw_bytes[..n]) {
            let hwnd = hwnd_raw as HWND;
            // SAFETY: `PostMessageW` es seguro entre hilos; si la ventana ya cerró, simplemente devuelve 0.
            unsafe {
                PostMessageW(hwnd, WM_CONFIG_CHANGED, 0, 0);
            }
        }
    }

    // SAFETY: `handle` es válido y ya no se usa.
    unsafe {
        CloseHandle(handle);
    }
}

/// Recorre de forma segura las entradas `FILE_NOTIFY_INFORMATION` devueltas por
/// `ReadDirectoryChangesW` y devuelve `true` si alguna corresponde a `config.ini`.
fn mentions_config_file(mut slice: &[u8]) -> bool {
    const HEADER_SIZE: usize = std::mem::offset_of!(FILE_NOTIFY_INFORMATION, FileName);
    while slice.len() >= HEADER_SIZE {
        let Some(next_bytes) = slice.get(0..4).and_then(|s| <[u8; 4]>::try_from(s).ok()) else {
            break;
        };
        let Some(len_bytes) = slice.get(8..12).and_then(|s| <[u8; 4]>::try_from(s).ok()) else {
            break;
        };
        let next_offset = u32::from_ne_bytes(next_bytes) as usize;
        let name_byte_len = u32::from_ne_bytes(len_bytes) as usize;

        if let Some(name_bytes) = slice.get(HEADER_SIZE..HEADER_SIZE.saturating_add(name_byte_len))
        {
            if is_config_ini_utf16(name_bytes) {
                return true;
            }
        }

        if next_offset == 0 || next_offset >= slice.len() {
            break;
        }
        slice = &slice[next_offset..];
    }
    false
}

/// Comprueba si `name_bytes` (UTF-16LE) es exactamente `"config.ini"` sin distinguir mayúsculas.
fn is_config_ini_utf16(name_bytes: &[u8]) -> bool {
    let expected = CONFIG_FILE_NAME.as_bytes();
    if name_bytes.len() != expected.len() * 2 {
        return false;
    }
    let (pairs, remainder) = name_bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return false;
    }
    for (&pair, &exp) in pairs.iter().zip(expected.iter()) {
        let unit = u16::from_ne_bytes(pair);
        let Ok(byte) = u8::try_from(unit) else {
            return false;
        };
        if !byte.eq_ignore_ascii_case(&exp) {
            return false;
        }
    }
    true
}

/// Entrega la imagen a Windows como contenido de la ventana (alfa por píxel).
///
/// También sirve para cambiar el tamaño y la posición de la ventana: Windows
/// toma ambos de los parámetros de esta llamada.
///
/// # Safety
///
/// `hwnd` debe ser una ventana válida con `WS_EX_LAYERED`, propiedad de este hilo.
unsafe fn present_frame(
    hwnd: HWND,
    left: i32,
    top: i32,
    frame: &Frame,
) -> Result<(), PlatformError> {
    let size = frame.size();
    let width = i32::try_from(size.width).map_err(|_| PlatformError::InvalidFrame)?;
    let height = i32::try_from(size.height).map_err(|_| PlatformError::InvalidFrame)?;
    let pixels = frame.bgra_premultiplied();
    if pixels.len() != size.width as usize * size.height as usize * 4 {
        return Err(PlatformError::InvalidFrame);
    }

    // SAFETY: con ventana nula devuelve el contexto de dispositivo de la pantalla.
    let screen_dc = unsafe { GetDC(null_mut()) };
    if screen_dc.is_null() {
        return Err(os_error("GetDC"));
    }
    // SAFETY: `screen_dc` es válido.
    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if memory_dc.is_null() {
        let err = os_error("CreateCompatibleDC");
        // SAFETY: `screen_dc` fue obtenido con `GetDC(null)`.
        unsafe { ReleaseDC(null_mut(), screen_dc) };
        return Err(err);
    }

    // SAFETY: `BITMAPINFO` es de datos planos; cero es un valor válido.
    let mut info: BITMAPINFO = unsafe { zeroed() };
    info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width;
    // Alto negativo: las filas van de arriba hacia abajo, igual que el `Frame`.
    info.bmiHeader.biHeight = -height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;

    let mut bits: *mut c_void = null_mut();
    // SAFETY: `info` describe un bitmap de 32 bits y `bits` recibe su memoria.
    let bitmap =
        unsafe { CreateDIBSection(screen_dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };

    let result = if bitmap.is_null() || bits.is_null() {
        Err(os_error("CreateDIBSection"))
    } else {
        // SAFETY: `bits` apunta a `width * height * 4` bytes y `pixels` tiene
        // exactamente esa cantidad (verificado arriba); no se superponen.
        unsafe { copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len()) };

        let origin = POINT { x: left, y: top };
        let extent = SIZE {
            cx: width,
            cy: height,
        };
        let source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        // SAFETY: todos los punteros apuntan a variables locales vivas y los
        // contextos de dispositivo son válidos.
        unsafe {
            let previous = SelectObject(memory_dc, bitmap);
            let ok = UpdateLayeredWindow(
                hwnd, screen_dc, &origin, &extent, memory_dc, &source, 0, &blend, ULW_ALPHA,
            );
            let outcome = if ok == 0 {
                Err(os_error("UpdateLayeredWindow"))
            } else {
                Ok(())
            };
            SelectObject(memory_dc, previous);
            outcome
        }
    };

    // SAFETY: se liberan los recursos creados arriba, una sola vez cada uno.
    unsafe {
        if !bitmap.is_null() {
            DeleteObject(bitmap);
        }
        DeleteDC(memory_dc);
        ReleaseDC(null_mut(), screen_dc);
    }
    result
}

/// El mouse entró o se movió dentro de la isla.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_mouse_move(hwnd: HWND) {
    let Some(mut animator) = ANIMATOR.get() else {
        return;
    };
    let was_inside = animator.is_pointer_inside();
    let action = animator.on_mouse_enter(Instant::now());
    ANIMATOR.set(Some(animator));

    if !was_inside {
        // Pide el aviso `WM_MOUSELEAVE` para saber cuándo sale el mouse, incluso si
        // estamos esperando que venza el timer del retardo (`HOVER_DELAY_TIMER`).
        let mut request = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        // SAFETY: `request` está completo y `hwnd` es válido.
        unsafe { TrackMouseEvent(&mut request) };
    }

    // SAFETY: `hwnd` es válido.
    unsafe { execute_hover_action(hwnd, action) };
}

/// El mouse salió de la isla: cancela el retardo pendiente o contrae sin retardo.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_mouse_leave(hwnd: HWND) {
    let Some(mut animator) = ANIMATOR.get() else {
        return;
    };
    let action = animator.on_mouse_leave(Instant::now());
    ANIMATOR.set(Some(animator));

    // SAFETY: `hwnd` es válido.
    unsafe { execute_hover_action(hwnd, action) };
}

/// Venció el timer de un solo disparo del retardo de hover (`HOVER_DELAY_TIMER`).
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_delay_timer(hwnd: HWND) {
    // Es de un solo disparo: se apaga inmediatamente al disparar.
    // SAFETY: `hwnd` es válido.
    unsafe { KillTimer(hwnd, HOVER_DELAY_TIMER) };

    let Some(mut animator) = ANIMATOR.get() else {
        return;
    };
    let action = animator.on_delay_elapsed(Instant::now());
    ANIMATOR.set(Some(animator));

    // SAFETY: `hwnd` es válido.
    unsafe { execute_hover_action(hwnd, action) };
}

/// Ejecuta en Win32 la acción pura devuelta por la máquina de estados del `Animator`.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn execute_hover_action(hwnd: HWND, action: HoverAction) {
    match action {
        HoverAction::None => {}
        HoverAction::ArmDelayTimer { delay_ms } => {
            // SAFETY: `hwnd` es válido; se apaga al vencer o si el mouse sale antes.
            unsafe { SetTimer(hwnd, HOVER_DELAY_TIMER, delay_ms, None) };
        }
        HoverAction::CancelDelayTimer => {
            // SAFETY: `hwnd` es válido.
            unsafe { KillTimer(hwnd, HOVER_DELAY_TIMER) };
        }
        HoverAction::StartExpand | HoverAction::StartCollapse => {
            // SAFETY: `hwnd` es válido; nos aseguramos de que no quede un retardo colgado.
            unsafe { KillTimer(hwnd, HOVER_DELAY_TIMER) };
            let instant = ANIMATOR.get().is_some_and(|a| a.duration().is_zero());
            if instant {
                // Con `animation_ms = 0`, el cambio es instantáneo y no se arma timer.
                // SAFETY: `hwnd` es válido.
                unsafe { on_timer(hwnd) };
            } else {
                // SAFETY: `hwnd` es válido; el timer se apaga solo al terminar la animación.
                unsafe { SetTimer(hwnd, ANIMATION_TIMER, FRAME_INTERVAL_MS, None) };
            }
        }
    }
}

/// Un tick del timer: dibuja el cuadro siguiente y apaga el timer al terminar.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_timer(hwnd: HWND) {
    let Some(mut animator) = ANIMATOR.get() else {
        // SAFETY: `hwnd` es válido.
        unsafe { KillTimer(hwnd, ANIMATION_TIMER) };
        return;
    };
    let step = animator.tick(Instant::now());
    ANIMATOR.set(Some(animator));

    let drawn = draw_step(hwnd, step.size);
    if step.finished || !drawn {
        // SAFETY: `hwnd` es válido. Si falló el dibujo también se apaga el
        // timer, para no repetir el error sesenta veces por segundo.
        unsafe { KillTimer(hwnd, ANIMATION_TIMER) };
    }
}

/// Le pide el cuadro a quien dibuja, para el tamaño dado y la hora actual.
fn paint(size: Size) -> Option<(Frame, LocalTime)> {
    let time = local_time().ok()?;
    let mut painter = PAINTER.take()?;
    let frame = painter.paint(size, SCALE.get(), time);
    PAINTER.set(Some(painter));
    frame.map(|frame| (frame, time))
}

/// Dibuja la isla con el tamaño dado y la entrega a Windows. `false` si falla.
fn draw_step(hwnd: HWND, size: Size) -> bool {
    let Some((frame, time)) = paint(size) else {
        return false;
    };
    let (left, top) = island_origin(size.width);
    // SAFETY: `hwnd` es la ventana de la isla, válida y de este hilo.
    let presented = unsafe { present_frame(hwnd, left, top, &frame) }.is_ok();
    if presented {
        LAST_DRAWN.set(Some(time));
    }
    presented
}

/// Arma el timer del reloj para el próximo cambio de minuto.
///
/// Se usa `SetCoalescableTimer` sin coalescencia: con el `SetTimer` común Windows
/// puede agrupar timers y demorar el disparo, y el reloj cambiaría tarde.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, propiedad de este hilo.
unsafe fn arm_clock(hwnd: HWND) {
    let wait = local_time()
        .map_or(CLOCK_FALLBACK_MS, |now| now.millis_until_next_minute())
        .saturating_add(CLOCK_MARGIN_MS);
    // SAFETY: `hwnd` es válido; sin función de callback, el aviso llega como `WM_TIMER`.
    let armed = unsafe { SetCoalescableTimer(hwnd, CLOCK_TIMER, wait, None, TIMERV_NO_COALESCING) };
    if armed == 0 {
        // Respaldo: si Windows rechaza el timer sin coalescencia, se usa el común.
        // SAFETY: ídem.
        unsafe { SetTimer(hwnd, CLOCK_TIMER, wait, None) };
    }
}

/// Disparo del timer del reloj, o cambio de hora del sistema, o regreso de suspensión.
///
/// Es de un solo disparo: apaga el timer, redibuja solo si el minuto cambió y
/// vuelve a armarlo con los milisegundos que faltan.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_clock(hwnd: HWND) {
    // SAFETY: `hwnd` es válido.
    unsafe { KillTimer(hwnd, CLOCK_TIMER) };

    if let Ok(now) = local_time() {
        let stale = LAST_DRAWN
            .get()
            .is_none_or(|drawn| !drawn.same_minute(&now));
        if stale {
            let size = ANIMATOR.get().map(|animator| animator.size());
            if let Some(size) = size {
                draw_step(hwnd, size);
            }
        }
    }

    // SAFETY: `hwnd` es válido.
    unsafe { arm_clock(hwnd) };
}

/// Aplica un factor de escala y las medidas lógicas: guarda el factor, escala el
/// margen y reconfigura los tamaños físicos y tiempos del animador conservando el hover.
fn apply_scale(metrics: LogicalMetrics, scale: f32) {
    SCALE.set(scale);
    TOP_MARGIN.set(to_i32(scale_px(metrics.top_margin, scale)));
    let collapsed = scale_size(metrics.size, scale);
    let expanded = scale_size(metrics.hover_size, scale);
    let duration = Duration::from_millis(u64::from(metrics.animation_ms));
    let animator = match ANIMATOR.get() {
        Some(mut animator) => {
            animator.set_timing(metrics.hover_delay_ms, duration);
            animator.reconfigure(collapsed, expanded);
            animator
        }
        None => Animator::new(collapsed, expanded, duration, Instant::now())
            .with_timing(metrics.hover_delay_ms, duration),
    };
    ANIMATOR.set(Some(animator));
}

/// Cambió el DPI del monitor, o la disposición de pantallas: se recalcula la escala
/// y se redibuja una sola vez, ya en el tamaño y la posición nuevos.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_dpi_changed(hwnd: HWND, dpi: u32) {
    let Some(metrics) = CONFIG.get() else {
        return;
    };
    apply_scale(metrics, scale_for_dpi(dpi));
    // El cambio de tamaño es instantáneo: si había una animación o retardo en curso, se apagan.
    // SAFETY: `hwnd` es válido.
    unsafe {
        KillTimer(hwnd, ANIMATION_TIMER);
        KillTimer(hwnd, HOVER_DELAY_TIMER);
    }
    if let Some(animator) = ANIMATOR.get() {
        draw_step(hwnd, animator.size());
    }
}

/// Disparo del timer de debounce (50 ms después del último cambio en `config.ini`).
///
/// Es de un solo disparo: apaga el timer, le pide al `Painter` que relea la
/// configuración y, solo si cambió respecto de la actual, reconfigura el
/// `Animator`, apaga la animación en curso y redibuja una sola vez.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_reload_timer(hwnd: HWND) {
    // SAFETY: `hwnd` es válido.
    unsafe { KillTimer(hwnd, RELOAD_TIMER) };

    let reloaded = if let Some(mut painter) = PAINTER.take() {
        let updated = painter.reload_config();
        PAINTER.set(Some(painter));
        updated
    } else {
        None
    };

    let Some(new_config) = reloaded else {
        return;
    };

    let metrics = LogicalMetrics::from(&new_config);
    CONFIG.set(Some(metrics));
    apply_scale(metrics, SCALE.get());
    // SAFETY: `hwnd` es válido.
    unsafe {
        KillTimer(hwnd, ANIMATION_TIMER);
        KillTimer(hwnd, HOVER_DELAY_TIMER);
    }
    if let Some(animator) = ANIMATOR.get() {
        draw_step(hwnd, animator.size());
    }
}

/// Procedimiento de ventana: reacciona a cada evento que manda Windows.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_MOUSEMOVE => {
            // SAFETY: Windows invoca este procedimiento con un `hwnd` válido.
            unsafe { on_mouse_move(hwnd) };
            0
        }
        WM_MOUSELEAVE => {
            // SAFETY: ídem.
            unsafe { on_mouse_leave(hwnd) };
            0
        }
        WM_TIMER if wparam == ANIMATION_TIMER => {
            // SAFETY: ídem.
            unsafe { on_timer(hwnd) };
            0
        }
        WM_TIMER if wparam == CLOCK_TIMER => {
            // SAFETY: ídem.
            unsafe { on_clock(hwnd) };
            0
        }
        WM_TIMER if wparam == RELOAD_TIMER => {
            // SAFETY: ídem.
            unsafe { on_reload_timer(hwnd) };
            0
        }
        WM_TIMER if wparam == HOVER_DELAY_TIMER => {
            // SAFETY: ídem.
            unsafe { on_delay_timer(hwnd) };
            0
        }
        WM_CONFIG_CHANGED => {
            // Arma o reinicia el timer de debounce de 50 ms (un solo disparo).
            // SAFETY: `hwnd` es válido.
            unsafe { SetTimer(hwnd, RELOAD_TIMER, RELOAD_DEBOUNCE_MS, None) };
            0
        }
        // Cambió el DPI del monitor (el DPI nuevo viene en la palabra baja de `wparam`).
        WM_DPICHANGED => {
            let dpi = u32::try_from(wparam & 0xFFFF).unwrap_or(0);
            // SAFETY: ídem.
            unsafe { on_dpi_changed(hwnd, dpi) };
            0
        }
        // Cambió la resolución o la disposición de monitores: se reubica la isla.
        WM_DISPLAYCHANGE => {
            // SAFETY: ídem.
            unsafe { on_dpi_changed(hwnd, GetDpiForWindow(hwnd)) };
            0
        }
        // Cambió la hora del sistema: se re-sincroniza el reloj.
        WM_TIMECHANGE => {
            // SAFETY: ídem.
            unsafe { on_clock(hwnd) };
            0
        }
        // La PC volvió de suspensión: el timer pudo quedar desfasado.
        WM_POWERBROADCAST
            if wparam == PBT_APMRESUMEAUTOMATIC as usize
                || wparam == PBT_APMRESUMESUSPEND as usize =>
        {
            // SAFETY: ídem.
            unsafe { on_clock(hwnd) };
            1
        }
        // Clic derecho sobre la isla: cerrar.
        WM_RBUTTONUP => {
            // SAFETY: `hwnd` es válido y pertenece a este hilo.
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            WINDOW_CLOSED.store(true, Ordering::SeqCst);
            // SAFETY: sin precondiciones; termina el loop de mensajes.
            unsafe {
                KillTimer(hwnd, ANIMATION_TIMER);
                KillTimer(hwnd, CLOCK_TIMER);
                KillTimer(hwnd, RELOAD_TIMER);
                KillTimer(hwnd, HOVER_DELAY_TIMER);
                PostQuitMessage(0);
            }
            0
        }
        // SAFETY: delegamos el resto al procedimiento por defecto de Windows.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::mentions_config_file;

    /// Arma una entrada `FILE_NOTIFY_INFORMATION` en bytes para probar el filtro.
    fn encode_entry(next_offset: u32, action: u32, name: &str) -> Vec<u8> {
        let utf16: Vec<u16> = name.encode_utf16().collect();
        let name_byte_len = (utf16.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(&next_offset.to_ne_bytes());
        out.extend_from_slice(&action.to_ne_bytes());
        out.extend_from_slice(&name_byte_len.to_ne_bytes());
        for unit in utf16 {
            out.extend_from_slice(&unit.to_ne_bytes());
        }
        out
    }

    #[test]
    fn matches_config_ini_case_insensitively_and_ignores_other_files() {
        let hit = encode_entry(0, 3, "config.ini");
        assert!(mentions_config_file(&hit));

        let upper = encode_entry(0, 1, "CONFIG.INI");
        assert!(mentions_config_file(&upper));

        let other = encode_entry(0, 3, "otro.txt");
        assert!(!mentions_config_file(&other));
    }

    #[test]
    fn finds_config_ini_in_chained_temp_rename_notifications() {
        let mut first = encode_entry(0, 4, "config.ini.tmp");
        while (first.len() & 3) != 0 {
            first.push(0);
        }
        let offset = first.len() as u32;
        first[0..4].copy_from_slice(&offset.to_ne_bytes());

        let second = encode_entry(0, 5, "config.ini");
        first.extend_from_slice(&second);
        assert!(mentions_config_file(&first));
    }
}
