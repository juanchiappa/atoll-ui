//! Backend de Windows (Win32).
//!
//! Loop orientado a eventos: `GetMessageW` bloquea el hilo hasta que llega un
//! mensaje, así que en reposo el consumo de CPU es 0%. La isla es una ventana
//! con transparencia por píxel (`WS_EX_LAYERED`): la imagen se entrega una sola
//! vez a Windows con `UpdateLayeredWindow` y el sistema la mantiene en pantalla
//! sin que Atoll haga nada más.

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{copy_nonoverlapping, null, null_mut};

use atoll_render::Frame;
use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, LoadCursorW, PostQuitMessage, RegisterClassExW, SetProcessDPIAware,
    ShowWindow, TranslateMessage, UnregisterClassW, UpdateLayeredWindow, IDC_ARROW, MSG,
    SM_CXSCREEN, SW_SHOWNOACTIVATE, ULW_ALPHA, WM_DESTROY, WM_RBUTTONUP, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::{Backend, IslandConfig, PlatformError};

/// Nombre de la clase de ventana registrada en Win32.
const CLASS_NAME: &str = "AtollIsland";

/// Backend nativo para Windows.
pub(crate) struct WindowsBackend;

impl Backend for WindowsBackend {
    fn run(&mut self, config: &IslandConfig, frame: &Frame) -> Result<(), PlatformError> {
        let size = frame.size();
        if size.width != config.width || size.height != config.height {
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

/// Abre la isla y corre el loop de mensajes hasta que se cierra.
///
/// # Safety
///
/// Debe llamarse desde un único hilo, que pasa a ser el dueño de la ventana.
unsafe fn run_island(config: &IslandConfig, frame: &Frame) -> Result<(), PlatformError> {
    let width = i32::try_from(config.width).map_err(|_| PlatformError::InvalidFrame)?;
    let height = i32::try_from(config.height).map_err(|_| PlatformError::InvalidFrame)?;
    let top = i32::try_from(config.top_margin).unwrap_or(0);

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

    // SAFETY: sin precondiciones.
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let left = (screen_width - width).max(0) / 2;

    // SAFETY: la clase ya está registrada y los punteros son válidos.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            left,
            top,
            width,
            height,
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

    // SAFETY: la ventana ya fue destruida; se libera la clase registrada.
    unsafe { UnregisterClassW(class_name.as_ptr(), hinstance) };
    result
}

/// Entrega la imagen a Windows como contenido de la ventana (alfa por píxel).
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

/// Procedimiento de ventana: reacciona a cada evento que manda Windows.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        // Clic derecho sobre la isla: cerrar.
        WM_RBUTTONUP => {
            // SAFETY: `hwnd` es válido y pertenece a este hilo.
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            // SAFETY: sin precondiciones; termina el loop de mensajes.
            unsafe { PostQuitMessage(0) };
            0
        }
        // SAFETY: delegamos el resto al procedimiento por defecto de Windows.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
