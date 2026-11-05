//! Backend de Windows (Win32).
//!
//! Loop orientado a eventos: `GetMessageW` bloquea el hilo hasta que llega un
//! mensaje, así que en reposo el consumo de CPU es 0%. La isla solo se dibuja
//! cuando Windows manda `WM_PAINT`.

use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
    SetWindowRgn, PAINTSTRUCT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, LoadCursorW, PostQuitMessage, RegisterClassExW, SetProcessDPIAware,
    ShowWindow, TranslateMessage, UnregisterClassW, IDC_ARROW, MSG, SM_CXSCREEN, SW_SHOWNOACTIVATE,
    WM_DESTROY, WM_ERASEBKGND, WM_PAINT, WM_RBUTTONUP, WNDCLASSEXW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::{Backend, IslandConfig, PlatformError};

/// Nombre de la clase de ventana registrada en Win32.
const CLASS_NAME: &str = "AtollIsland";

/// Color de la isla (COLORREF 0x00BBGGRR): negro puro.
const ISLAND_COLOR: u32 = 0x0000_0000;

/// Backend nativo para Windows.
pub(crate) struct WindowsBackend;

impl Backend for WindowsBackend {
    fn run(&mut self, config: &IslandConfig) -> Result<(), PlatformError> {
        // SAFETY: todas las llamadas Win32 ocurren en este hilo, que es el
        // dueño de la ventana y de su cola de mensajes.
        unsafe { run_island(config) }
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
unsafe fn run_island(config: &IslandConfig) -> Result<(), PlatformError> {
    let width = i32::try_from(config.width).unwrap_or(i32::MAX);
    let height = i32::try_from(config.height).unwrap_or(i32::MAX);
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
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
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

    // SAFETY: `hwnd` es válido. Si la región se crea, Windows pasa a ser su
    // dueño y no hay que liberarla; si falla, la isla queda rectangular.
    unsafe {
        let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, height, height);
        if !region.is_null() {
            SetWindowRgn(hwnd, region, 1);
        }
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
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

/// Procedimiento de ventana: reacciona a cada evento que manda Windows.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            // SAFETY: Windows invoca este procedimiento con un `hwnd` válido.
            unsafe { paint(hwnd) };
            0
        }
        // Evita el parpadeo: pintamos todo en WM_PAINT.
        WM_ERASEBKGND => 1,
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

/// Dibuja la isla: un rectángulo negro (la región redondeada la recorta).
///
/// # Safety
///
/// `hwnd` debe ser una ventana válida de este hilo, dentro de `WM_PAINT`.
unsafe fn paint(hwnd: HWND) {
    // SAFETY: `PAINTSTRUCT` es de datos planos; cero es un valor válido.
    let mut ps: PAINTSTRUCT = unsafe { zeroed() };
    // SAFETY: `hwnd` es válido y estamos dentro de `WM_PAINT`.
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc.is_null() {
        return;
    }
    // SAFETY: el pincel se crea, se usa y se libera acá mismo.
    unsafe {
        let brush = CreateSolidBrush(ISLAND_COLOR);
        if !brush.is_null() {
            FillRect(hdc, &ps.rcPaint, brush);
            DeleteObject(brush);
        }
        EndPaint(hwnd, &ps);
    }
}
