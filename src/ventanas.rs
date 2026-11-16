//! Ventanas de otros programas: a qué ejecutable pertenecen, cuáles están abiertas y cómo traer
//! una al frente.
//!
//! Sirve para tres cosas: las exclusiones y los grupos por programa (qué hay delante al pulsar el
//! botón), la lista de "programas abiertos" de los ajustes, y enfocar la ventana de un acceso que
//! ya está abierto en lugar de lanzar otra copia.

use crate::config::expandir;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Storage::Packaging::Appx::GetApplicationUserModelId;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::*;

/// A qué ventanas corresponde el destino de un acceso.
#[derive(Debug, PartialEq, Eq)]
pub enum Objetivo {
    /// Ruta completa del ejecutable, en minúsculas.
    Ruta(String),
    /// Solo el nombre del ejecutable (`notepad.exe`), en minúsculas.
    Exe(String),
    /// Identificador de una app empaquetada (Microsoft Store), en minúsculas.
    Aumid(String),
}

/// Qué ventanas buscar para un destino; `None` si no se puede saber (una web, una carpeta, un
/// documento, un acceso directo): esos se abren siempre.
pub fn objetivo(destino: &str) -> Option<Objetivo> {
    let d = expandir(destino.trim()).trim_matches('"').to_lowercase();
    if let Some(id) = d.strip_prefix("shell:appsfolder\\") {
        // Las apps de la Store se nombran `Familia!App`; las de escritorio, con la ruta de su
        // ejecutable (a veces con el GUID de una carpeta conocida delante).
        if id.contains('!') {
            return Some(Objetivo::Aumid(id.to_string()));
        }
        return id.ends_with(".exe").then(|| Objetivo::Exe(nombre_de(id).to_string()));
    }
    if !d.ends_with(".exe") || d.contains("://") {
        return None;
    }
    if std::path::Path::new(&d).is_absolute() {
        Some(Objetivo::Ruta(d))
    } else if !d.contains(['\\', '/']) {
        Some(Objetivo::Exe(d))
    } else {
        None
    }
}

fn nombre_de(ruta: &str) -> &str {
    ruta.rsplit(['\\', '/']).next().unwrap_or(ruta)
}

struct Proceso(HANDLE);

impl Proceso {
    fn de_ventana(ventana: HWND) -> Option<(Self, u32)> {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(ventana, Some(&mut pid));
            let asa = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            Some((Proceso(asa), pid))
        }
    }

    /// Ruta completa del ejecutable, en minúsculas.
    fn ruta(&self) -> Option<String> {
        let mut ruta = [0u16; 1024];
        let mut largo = ruta.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(self.0, PROCESS_NAME_WIN32, PWSTR(ruta.as_mut_ptr()), &mut largo).ok()?;
        }
        Some(String::from_utf16_lossy(&ruta[..largo as usize]).to_lowercase())
    }

    /// Identificador de app empaquetada, en minúsculas; `None` en un programa de escritorio.
    fn aumid(&self) -> Option<String> {
        let mut id = [0u16; 256];
        let mut largo = id.len() as u32;
        let r = unsafe { GetApplicationUserModelId(self.0, &mut largo, Some(PWSTR(id.as_mut_ptr()))) };
        if r.0 != 0 || largo == 0 {
            return None;
        }
        // El largo incluye el cero final.
        Some(String::from_utf16_lossy(&id[..largo as usize - 1]).to_lowercase())
    }
}

impl Drop for Proceso {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Nombre del ejecutable de una ventana, en minúsculas (`blender.exe`).
pub fn ejecutable_de(ventana: HWND) -> Option<String> {
    let (proceso, _) = Proceso::de_ventana(ventana)?;
    proceso.ruta().map(|r| nombre_de(&r).to_string())
}

/// Nombre del ejecutable del programa que está delante ahora mismo.
pub fn ejecutable_activo() -> Option<String> {
    let activa = unsafe { GetForegroundWindow() };
    if activa.is_invalid() {
        return None;
    }
    ejecutable_de(activa)
}

fn clase(ventana: HWND) -> String {
    let mut clase = [0u16; 64];
    let n = unsafe { GetClassNameW(ventana, &mut clase) } as usize;
    String::from_utf16_lossy(&clase[..n])
}

/// ¿Es una ventana de aplicación, de las que salen en la barra de tareas? Visible, sin dueña, con
/// título, que no sea una ventana de herramientas ni una de las del escritorio, y que no esté
/// "oculta por el compositor" (apps de la Store suspendidas, otros escritorios virtuales).
fn es_de_aplicacion(ventana: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(ventana).as_bool() || GetWindowTextLengthW(ventana) == 0 {
            return false;
        }
        if GetWindow(ventana, GW_OWNER).is_ok_and(|d| !d.is_invalid()) {
            return false;
        }
        if GetWindowLongW(ventana, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }
        let mut oculta = 0u32;
        let _ = DwmGetWindowAttribute(ventana, DWMWA_CLOAKED, &mut oculta as *mut u32 as *mut _, 4);
        if oculta != 0 {
            return false;
        }
    }
    !matches!(clase(ventana).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd")
}

/// Ventanas de aplicación, de la de más arriba a la de más abajo.
fn de_aplicacion() -> Vec<HWND> {
    unsafe extern "system" fn anotar(ventana: HWND, lparam: LPARAM) -> BOOL {
        let lista = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        lista.push(ventana);
        BOOL(1)
    }
    let mut todas: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(anotar), LPARAM(&mut todas as *mut Vec<HWND> as isize));
    }
    todas.retain(|v| es_de_aplicacion(*v));
    todas
}

/// Las apps de la Store "clásicas" (UWP) viven dentro de una ventana de `ApplicationFrameHost`:
/// el proceso de la app es el de una ventana hija.
fn aumid_de_hijas(marco: HWND) -> Option<String> {
    unsafe extern "system" fn anotar(ventana: HWND, lparam: LPARAM) -> BOOL {
        let lista = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        lista.push(ventana);
        BOOL(1)
    }
    let mut hijas: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumChildWindows(Some(marco), Some(anotar), LPARAM(&mut hijas as *mut Vec<HWND> as isize));
    }
    hijas
        .into_iter()
        .filter(|h| clase(*h) == "Windows.UI.Core.CoreWindow")
        .find_map(|h| Proceso::de_ventana(h)?.0.aumid())
}

/// La ventana más reciente del programa de un acceso, si tiene alguna abierta.
pub fn buscar(destino: &str) -> Option<HWND> {
    let objetivo = objetivo(destino)?;
    let propio = unsafe { GetCurrentProcessId() };
    de_aplicacion().into_iter().find(|&ventana| {
        let Some((proceso, pid)) = Proceso::de_ventana(ventana) else {
            return false;
        };
        if pid == propio {
            return false;
        }
        match &objetivo {
            Objetivo::Ruta(ruta) => proceso.ruta().as_deref() == Some(ruta),
            Objetivo::Exe(exe) => proceso.ruta().is_some_and(|r| nombre_de(&r) == exe),
            Objetivo::Aumid(id) => {
                let directo = proceso.aumid();
                let encontrado = if clase(ventana) == "ApplicationFrameWindow" {
                    aumid_de_hijas(ventana).or(directo)
                } else {
                    directo
                };
                encontrado.as_deref() == Some(id)
            }
        }
    })
}

/// Trae una ventana al frente, restaurándola si estaba minimizada.
pub fn traer_al_frente(ventana: HWND) {
    unsafe {
        if IsIconic(ventana).as_bool() {
            let _ = ShowWindow(ventana, SW_RESTORE);
        }
        // Windows solo deja cambiar la ventana activa al programa que ya lo es. Orbyt nunca lo
        // es (la rueda no toma el foco), así que se une un momento a la cola de entrada del que
        // está delante.
        let delante = GetForegroundWindow();
        let hilo_delante = GetWindowThreadProcessId(delante, None);
        let hilo = GetCurrentThreadId();
        let unido = hilo_delante != 0 && hilo_delante != hilo && AttachThreadInput(hilo, hilo_delante, true).as_bool();
        let _ = BringWindowToTop(ventana);
        let _ = SetForegroundWindow(ventana);
        if unido {
            let _ = AttachThreadInput(hilo, hilo_delante, false);
        }
    }
}

/// Ejecutables con alguna ventana abierta ahora, sin repetir y sin Orbyt, por orden alfabético.
pub fn programas_abiertos() -> Vec<String> {
    let propio = unsafe { GetCurrentProcessId() };
    let mut lista: Vec<String> = Vec::new();
    for ventana in de_aplicacion() {
        let Some((proceso, pid)) = Proceso::de_ventana(ventana) else {
            continue;
        };
        if pid == propio {
            continue;
        }
        if let Some(exe) = proceso.ruta().map(|r| nombre_de(&r).to_string()) {
            // El marco de las apps de la Store no dice nada: es el mismo para todas.
            if exe != "applicationframehost.exe" && !lista.contains(&exe) {
                lista.push(exe);
            }
        }
    }
    lista.sort();
    lista
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_los_programas_tienen_ventanas_que_buscar() {
        assert_eq!(objetivo("notepad.exe"), Some(Objetivo::Exe("notepad.exe".into())));
        assert_eq!(
            objetivo("\"C:\\Apps\\Mi App\\App.EXE\""),
            Some(Objetivo::Ruta("c:\\apps\\mi app\\app.exe".into()))
        );
        assert_eq!(
            objetivo("shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            Some(Objetivo::Aumid("microsoft.windowscalculator_8wekyb3d8bbwe!app".into()))
        );
        assert_eq!(
            objetivo("shell:AppsFolder\\{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\notepad.exe"),
            Some(Objetivo::Exe("notepad.exe".into()))
        );
        assert_eq!(objetivo("shell:AppsFolder\\Chrome"), None);
        assert_eq!(objetivo("https://example.com/descarga.exe"), None);
        assert_eq!(objetivo("ms-settings:"), None);
        assert_eq!(objetivo("%USERPROFILE%\\Downloads"), None);
        assert_eq!(objetivo("C:\\notas.lnk"), None);
        assert_eq!(objetivo("bin\\app.exe"), None);
    }
}
