//! Ícono en el área de notificación. El clic izquierdo abre los ajustes y el derecho, el menú
//! propio de `menu.rs`. También es por donde Orbyt avisa de que algo falló: no tiene consola ni
//! ventana propia, así que sin esto un acceso que no abre solo se ve como que "no pasa nada".

use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_ORBYT_TRAY: u32 = WM_APP + 10;

/// Ventana dueña del ícono; 0 mientras no se ha puesto.
static VENTANA: AtomicIsize = AtomicIsize::new(0);

/// Copia `texto` en un campo de tamaño fijo, cortando si no cabe y dejando el cero final.
fn copiar(destino: &mut [u16], texto: &str) {
    let limite = destino.len() - 1;
    let mut escritas = 0;
    for (hueco, unidad) in destino[..limite].iter_mut().zip(texto.encode_utf16()) {
        *hueco = unidad;
        escritas += 1;
    }
    destino[escritas..].fill(0);
}

fn datos(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    copiar(
        &mut nid.szTip,
        "Orbyt: mantén el botón de la rueda · clic para configurar",
    );
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
    VENTANA.store(hwnd.0 as isize, Ordering::Relaxed);
    unsafe {
        let mut nid = datos(hwnd);
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_ORBYT_TRAY;
        nid.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

pub fn quitar(hwnd: HWND) {
    VENTANA.store(0, Ordering::Relaxed);
    unsafe {
        let nid = datos(hwnd);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// Muestra una notificación de Windows junto al ícono. Si el ícono aún no está puesto (o Windows
/// tiene las notificaciones apagadas), no hace nada.
pub fn avisar(titulo: &str, texto: &str) {
    let ventana = VENTANA.load(Ordering::Relaxed);
    if ventana == 0 {
        return;
    }
    let mut nid = datos(HWND(ventana as *mut _));
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = NIIF_WARNING;
    copiar(&mut nid.szInfoTitle, titulo);
    copiar(&mut nid.szInfo, texto);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copiar_corta_y_deja_el_cero_final() {
        let mut campo = [1u16; 4];
        copiar(&mut campo, "abcdef");
        assert_eq!(campo, [b'a' as u16, b'b' as u16, b'c' as u16, 0]);
        let mut campo = [0u16; 8];
        copiar(&mut campo, "sí");
        assert_eq!(String::from_utf16_lossy(&campo[..2]), "sí");
        assert_eq!(campo[2], 0);
    }
}
