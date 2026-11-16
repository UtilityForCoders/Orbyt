//! Comprobación de versiones nuevas, solo cuando se pide desde "Acerca de".
//!
//! Pregunta a GitHub cuál es el último release publicado. **Qué sale del equipo:** una petición
//! HTTPS a `api.github.com` sin ningún dato del usuario ni de la configuración. Nunca se hace
//! sola ni se descarga nada: si hay versión nueva, se ofrece abrir su página en el navegador.

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

const REPOSITORIO: &str = "UtilityForCoders/focus";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Estado {
    /// Todavía no se ha preguntado.
    SinComprobar,
    Buscando,
    AlDia,
    /// Hay una versión más nueva: su número y la página desde la que se descarga.
    Disponible {
        version: String,
        pagina: String,
    },
    /// No hubo respuesta (sin red, o GitHub no contestó).
    SinRespuesta,
}

/// Los números de una versión (`v1.2.3` o `1.2.3`); `None` si no tiene esa forma.
fn numeros(version: &str) -> Option<Vec<u64>> {
    let v = version.trim().trim_start_matches(['v', 'V']);
    // Lo que sigue a un guion o un más (`-beta.1`, `+build`) no cuenta.
    let base = v.split(['-', '+']).next()?;
    let partes: Option<Vec<u64>> = base.split('.').map(|p| p.parse().ok()).collect();
    partes.filter(|p| !p.is_empty() && p.len() <= 4)
}

/// ¿Es `otra` una versión posterior a `actual`?
pub fn mas_nueva(actual: &str, otra: &str) -> bool {
    match (numeros(actual), numeros(otra)) {
        (Some(mut a), Some(mut b)) => {
            let largo = a.len().max(b.len());
            a.resize(largo, 0);
            b.resize(largo, 0);
            b > a
        }
        _ => false,
    }
}

/// Etiqueta del último release, sacada de la respuesta de GitHub. Solo se acepta si tiene forma
/// de versión: con ella se arma después una dirección que se abre en el navegador.
fn etiqueta(json: &[u8]) -> Option<String> {
    let valor: serde_json::Value = serde_json::from_slice(json).ok()?;
    let tag = valor.get("tag_name")?.as_str()?.trim();
    let limpia = tag.len() <= 40 && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    (limpia && numeros(tag).is_some()).then(|| tag.to_string())
}

fn estado_de(respuesta: Option<Vec<u8>>) -> Estado {
    let Some(tag) = respuesta.as_deref().and_then(etiqueta) else {
        return Estado::SinRespuesta;
    };
    if mas_nueva(env!("CARGO_PKG_VERSION"), &tag) {
        Estado::Disponible {
            version: tag.trim_start_matches(['v', 'V']).to_string(),
            pagina: format!("https://github.com/{REPOSITORIO}/releases/tag/{tag}"),
        }
    } else {
        Estado::AlDia
    }
}

/// Pregunta en segundo plano y avisa a `ventana` con `msg`; el `lParam` es un `Box<Estado>` que
/// se queda quien recibe el mensaje (`Box::from_raw`).
pub fn comprobar(ventana: HWND, msg: u32) {
    let ventana = ventana.0 as isize;
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let ruta = format!("/repos/{REPOSITORIO}/releases/latest");
        let estado = estado_de(crate::web::descargar("api.github.com", &ruta));
        let caja = Box::into_raw(Box::new(estado)) as isize;
        let enviado = unsafe { PostMessageW(Some(HWND(ventana as *mut _)), msg, WPARAM(0), LPARAM(caja)) }.is_ok();
        if !enviado {
            // La ventana ya se cerró: se libera la caja.
            drop(unsafe { Box::from_raw(caja as *mut Estado) });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_versiones_por_numeros() {
        assert!(mas_nueva("0.1.0", "v0.2.0"));
        assert!(mas_nueva("0.9.9", "0.10.0"));
        assert!(mas_nueva("1.0", "1.0.1"));
        assert!(!mas_nueva("0.2.0", "v0.2.0"));
        assert!(!mas_nueva("1.0.0", "0.9.9"));
        assert!(!mas_nueva("1.0.0", "no-es-version"));
        assert!(mas_nueva("1.0.0", "1.1.0-beta.1"));
    }

    #[test]
    fn solo_acepta_etiquetas_con_forma_de_version() {
        assert_eq!(etiqueta(br#"{"tag_name":"v1.2.3"}"#).as_deref(), Some("v1.2.3"));
        assert_eq!(etiqueta(br#"{"tag_name":"1.2.3/../../x"}"#), None);
        assert_eq!(etiqueta(br#"{"tag_name":"ultima"}"#), None);
        assert_eq!(etiqueta(br#"{"message":"Not Found"}"#), None);
        assert_eq!(etiqueta(b"<html>"), None);
    }

    #[test]
    fn la_respuesta_decide_el_estado() {
        assert_eq!(estado_de(None), Estado::SinRespuesta);
        assert_eq!(estado_de(Some(br#"{"tag_name":"v0.0.1"}"#.to_vec())), Estado::AlDia);
        assert_eq!(
            estado_de(Some(br#"{"tag_name":"v999.0.0"}"#.to_vec())),
            Estado::Disponible {
                version: "999.0.0".into(),
                pagina: "https://github.com/UtilityForCoders/focus/releases/tag/v999.0.0".into(),
            }
        );
    }
}
