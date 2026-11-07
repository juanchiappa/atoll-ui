//! Backend de Windows (Win32).
//!
//! Loop orientado a eventos: `GetMessageW` bloquea el hilo hasta que llega un
//! mensaje, así que en reposo el consumo de CPU es 0%. La isla es una ventana
//! con transparencia por píxel (`WS_EX_LAYERED`): la imagen se entrega a Windows
//! con `UpdateLayeredWindow` y el sistema la mantiene en pantalla sin que Atoll
//! haga nada más.
//!
//! La animación de hover usa un timer de Win32 que **solo existe mientras la
//! isla se está moviendo**: arranca cuando el mouse entra o sale y se apaga
//! apenas la animación termina.

use std::cell::Cell;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{copy_nonoverlapping, null, null_mut};
use std::time::{Duration, Instant};

use atoll_render::{render_island, Frame, Size};
use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassExW,
    SetProcessDPIAware, SetTimer, ShowWindow, TranslateMessage, UnregisterClassW,
    UpdateLayeredWindow, IDC_ARROW, MSG, SM_CXSCREEN, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_DESTROY,
    WM_MOUSEMOVE, WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::animation::Animator;
use crate::{Backend, IslandConfig, PlatformError};

/// Nombre de la clase de ventana registrada en Win32.
const CLASS_NAME: &str = "AtollIsland";

/// Mensaje que Windows envía cuando el mouse sale de la ventana (0x02A3).
///
/// Se define acá porque `windows-sys` lo ubica en el módulo `Controls`, y
/// activar esa feature solo por una constante agrega código que no usamos.
const WM_MOUSELEAVE: u32 = 0x02A3;

/// Identificador del timer de la animación.
const ANIMATION_TIMER: usize = 1;

/// Milisegundos entre cuadros de la animación (≈ 60 cuadros por segundo).
const FRAME_INTERVAL_MS: u32 = 16;

/// Duración de la animación de expandir y contraer.
const ANIMATION_DURATION: Duration = Duration::from_millis(180);

thread_local! {
    /// Estado de la animación. El loop de mensajes corre en un solo hilo, así
    /// que alcanza con un `Cell` (y evita pánicos por reentrada del `RefCell`).
    static ANIMATOR: Cell<Option<Animator>> = const { Cell::new(None) };
    /// Margen superior de la isla, en píxeles.
    static TOP_MARGIN: Cell<i32> = const { Cell::new(0) };
}

/// Backend nativo para Windows.
pub(crate) struct WindowsBackend;

impl Backend for WindowsBackend {
    fn run(&mut self, config: &IslandConfig, frame: &Frame) -> Result<(), PlatformError> {
        if frame.size() != config.size {
            return Err(PlatformError::InvalidFrame);
        }
        // SAFETY: todas las llamadas Win32 ocurren en este hilo, que es el
        // dueño de la ventana y de su cola de mensajes.
        unsafe { run_island(config, frame) }
    }
}

/// Convierte un `&str` a UTF-16 terminado en cero, como pide Win32.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
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
unsafe fn run_island(config: &IslandConfig, frame: &Frame) -> Result<(), PlatformError> {
    TOP_MARGIN.set(to_i32(config.top_margin));
    ANIMATOR.set(Some(Animator::new(
        config.size,
        config.hover_size,
        ANIMATION_DURATION,
        Instant::now(),
    )));

    // SAFETY: sin precondiciones; evita que Windows escale la ventana.
    unsafe { SetProcessDPIAware() };

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

    let (left, top) = island_origin(config.size.width);

    // SAFETY: la clase ya está registrada y los punteros son válidos.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            left,
            top,
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
    if let Err(err) = unsafe { present_frame(hwnd, left, top, frame) } {
        // SAFETY: `hwnd` es válido; la clase fue registrada arriba.
        unsafe {
            DestroyWindow(hwnd);
            UnregisterClassW(class_name.as_ptr(), hinstance);
        }
        return Err(err);
    }

    // SAFETY: `hwnd` es válido.
    unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };

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

    ANIMATOR.set(None);
    // SAFETY: la ventana ya fue destruida; se libera la clase registrada.
    unsafe { UnregisterClassW(class_name.as_ptr(), hinstance) };
    result
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

/// El mouse entró o salió de la isla: arranca la animación si el estado cambió.
///
/// # Safety
///
/// `hwnd` debe ser la ventana de la isla, dentro de su procedimiento de ventana.
unsafe fn on_hover(hwnd: HWND, hovering: bool) {
    let Some(mut animator) = ANIMATOR.get() else {
        return;
    };
    let changed = animator.set_hover(hovering, Instant::now());
    ANIMATOR.set(Some(animator));
    if !changed {
        return;
    }

    if hovering {
        // Pide el aviso `WM_MOUSELEAVE` para saber cuándo sale el mouse.
        let mut request = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        // SAFETY: `request` está completo y `hwnd` es válido.
        unsafe { TrackMouseEvent(&mut request) };
    }
    // SAFETY: `hwnd` es válido; el timer se apaga solo al terminar la animación.
    unsafe { SetTimer(hwnd, ANIMATION_TIMER, FRAME_INTERVAL_MS, None) };
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

/// Dibuja la isla con el tamaño dado y la entrega a Windows. `false` si falla.
fn draw_step(hwnd: HWND, size: Size) -> bool {
    let Ok(frame) = render_island(size) else {
        return false;
    };
    let (left, top) = island_origin(size.width);
    // SAFETY: `hwnd` es la ventana de la isla, válida y de este hilo.
    unsafe { present_frame(hwnd, left, top, &frame) }.is_ok()
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
            unsafe { on_hover(hwnd, true) };
            0
        }
        WM_MOUSELEAVE => {
            // SAFETY: ídem.
            unsafe { on_hover(hwnd, false) };
            0
        }
        WM_TIMER if wparam == ANIMATION_TIMER => {
            // SAFETY: ídem.
            unsafe { on_timer(hwnd) };
            0
        }
        // Clic derecho sobre la isla: cerrar.
        WM_RBUTTONUP => {
            // SAFETY: `hwnd` es válido y pertenece a este hilo.
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            // SAFETY: sin precondiciones; termina el loop de mensajes.
            unsafe {
                KillTimer(hwnd, ANIMATION_TIMER);
                PostQuitMessage(0);
            }
            0
        }
        // SAFETY: delegamos el resto al procedimiento por defecto de Windows.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
