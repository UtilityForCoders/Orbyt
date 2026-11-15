//! Ícono en el área de notificación. El clic izquierdo abre los ajustes y el derecho, el menú
//! propio de `menu.rs`.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_ORBYT_TRAY: u32 = WM_APP + 10;

fn datos(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    let tip: Vec<u16> = "Orbyt: mantén el botón central · clic para configurar"
        .encode_utf16()
        .collect();
    nid.szTip[..tip.len()].copy_from_slice(&tip);
    nid
}

/// Mensaje "TaskbarCreated": Windows lo anuncia a todas las ventanas cuando el Explorador se
/// reinicia y la bandeja es nueva. Hay que volver a poner el ícono o desaparece.
pub fn es_barra_creada(msg: u32) -> bool {
    use std::sync::OnceLock;
    static MENSAJE: OnceLock<u32> = OnceLock::new();
    let id = *MENSAJE.get_or_init(|| unsafe { RegisterWindowMessageW(windows::core::w!("TaskbarCreated")) });
    id != 0 && msg == id
}

pub fn agregar(hwnd: HWND) {
    unsafe {
        let mut nid = datos(hwnd);
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_ORBYT_TRAY;
        nid.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

pub fn quitar(hwnd: HWND) {
    unsafe {
        let nid = datos(hwnd);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}
