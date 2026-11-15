//! Abre un acceso con el shell: programas, carpetas, archivos, URLs y URIs pasan todos por
//! `ShellExecuteW("open")`, que ya sabe qué hacer con cada uno.

use crate::config::{expandir, Acceso};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL};

pub fn abrir(acceso: &Acceso) {
    let destino = HSTRING::from(expandir(acceso.destino.trim()));
    let argumentos = acceso.argumentos.as_deref().map(|a| HSTRING::from(expandir(a)));
    // Un programa arranca en su propia carpeta, como desde un acceso directo: muchos (portables,
    // juegos) buscan ahí sus archivos. Sin esto heredaría la carpeta de trabajo de Orbyt.
    let carpeta = carpeta_de(&expandir(acceso.destino.trim())).map(|c| HSTRING::from(c.as_os_str()));
    unsafe {
        // Orbyt nunca toma el foco (la rueda no se activa), así que Windows no dejaría que la
        // ventana nueva pase al frente. Esto le cede el permiso al proceso que se abre.
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let resultado = ShellExecuteW(
            None,
            w!("open"),
            &destino,
            argumentos.as_ref().map_or(PCWSTR::null(), |a| PCWSTR(a.as_ptr())),
            carpeta.as_ref().map_or(PCWSTR::null(), |c| PCWSTR(c.as_ptr())),
            SW_SHOWNORMAL,
        );
        // Según la documentación, un valor <= 32 es un código de error.
        if resultado.0 as isize <= 32 {
            eprintln!("no se pudo abrir {:?} (código {})", acceso.destino, resultado.0 as isize);
        }
    }
}

/// La carpeta de un destino que es un archivo en disco; `None` para URLs, URIs, carpetas, apps de
/// `shell:AppsFolder` y nombres sueltos que resuelve el PATH.
fn carpeta_de(destino: &str) -> Option<std::path::PathBuf> {
    let ruta = std::path::Path::new(destino.trim_matches('"'));
    if !ruta.is_absolute() || !ruta.is_file() {
        return None;
    }
    ruta.parent().map(std::path::Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_los_archivos_en_disco_tienen_carpeta_de_trabajo() {
        let exe = std::env::current_exe().unwrap();
        assert_eq!(carpeta_de(exe.to_str().unwrap()).as_deref(), exe.parent());
        assert_eq!(carpeta_de(&format!("\"{}\"", exe.display())).as_deref(), exe.parent());
        assert_eq!(carpeta_de("https://github.com"), None);
        assert_eq!(carpeta_de("ms-settings:"), None);
        assert_eq!(carpeta_de("notepad.exe"), None);
        assert_eq!(carpeta_de(exe.parent().unwrap().to_str().unwrap()), None);
    }
}
