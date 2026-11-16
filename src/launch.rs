//! Abre un acceso con el shell: programas, carpetas, archivos, URLs y URIs pasan todos por
//! `ShellExecuteW`, que ya sabe qué hacer con cada uno.

use crate::config::{expandir, Acceso};
use crate::{tray, ventanas};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOWNORMAL};

/// Abre un acceso. Con `enfocar`, si su programa ya tiene una ventana abierta se trae al frente
/// en lugar de lanzar otra copia (salvo que el acceso lleve argumentos: entonces pide algo
/// concreto, como abrir un archivo, y se lanza siempre).
///
/// Si no se puede abrir, avisa con una notificación: Orbyt no tiene otra forma de decirlo.
pub fn abrir(acceso: &Acceso, enfocar: bool) {
    let sin_argumentos = acceso.argumentos.as_deref().is_none_or(|a| a.trim().is_empty());
    if enfocar && sin_argumentos {
        if let Some(ventana) = ventanas::buscar(&acceso.destino) {
            ventanas::traer_al_frente(ventana);
            return;
        }
    }
    if let Err(motivo) = lanzar(acceso) {
        let nombre = if acceso.nombre.trim().is_empty() {
            acceso.destino.as_str()
        } else {
            acceso.nombre.as_str()
        };
        tray::avisar(&format!("No se pudo abrir {nombre}"), &motivo);
    }
}

fn lanzar(acceso: &Acceso) -> Result<(), String> {
    let expandido = expandir(acceso.destino.trim());
    let destino = HSTRING::from(expandido.as_str());
    let argumentos = acceso.argumentos.as_deref().map(|a| HSTRING::from(expandir(a)));
    // Un programa arranca en su propia carpeta, como desde un acceso directo: muchos (portables,
    // juegos) buscan ahí sus archivos. Sin esto heredaría la carpeta de trabajo de Orbyt.
    let carpeta = carpeta_de(&expandido).map(|c| HSTRING::from(c.as_os_str()));
    // "runas" pide a Windows la confirmación de administrador antes de abrir.
    let verbo = if acceso.admin { w!("runas") } else { w!("open") };
    let (resultado, error) = unsafe {
        // Orbyt nunca toma el foco (la rueda no se activa), así que Windows no dejaría que la
        // ventana nueva pase al frente. Esto le cede el permiso al proceso que se abre.
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let resultado = ShellExecuteW(
            None,
            verbo,
            &destino,
            argumentos.as_ref().map_or(PCWSTR::null(), |a| PCWSTR(a.as_ptr())),
            carpeta.as_ref().map_or(PCWSTR::null(), |c| PCWSTR(c.as_ptr())),
            SW_SHOWNORMAL,
        );
        (resultado.0 as isize, GetLastError())
    };
    // Según la documentación, un valor <= 32 es un código de error.
    if resultado > 32 {
        return Ok(());
    }
    // Decir que no en la confirmación de administrador no es un fallo.
    if acceso.admin && error == ERROR_CANCELLED {
        return Ok(());
    }
    Err(motivo(resultado, &expandido))
}

/// Explicación de un código de error de `ShellExecuteW`.
fn motivo(codigo: isize, destino: &str) -> String {
    let causa = match codigo {
        2 | 3 => "No se encuentra",
        5 => "Windows denegó el acceso a",
        8 => "No hay memoria suficiente para abrir",
        26 | 32 => "No se puede acceder ahora a",
        27..=31 => "No hay ningún programa para abrir",
        _ => "No se pudo abrir",
    };
    format!("{causa} «{destino}» (código {codigo}). Revísalo en Configuración → Accesos.")
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

    #[test]
    fn el_motivo_nombra_el_destino_y_la_causa() {
        let m = motivo(2, "C:\\no\\existe.exe");
        assert!(m.starts_with("No se encuentra «C:\\no\\existe.exe»"));
        assert!(motivo(31, "x.zzz").starts_with("No hay ningún programa"));
        assert!(motivo(0, "x").starts_with("No se pudo abrir"));
    }
}
