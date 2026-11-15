//! Ganchos globales de ratón y teclado (`WH_MOUSE_LL`, `WH_KEYBOARD_LL`).
//!
//! Hay que usar un gancho y no consultar el estado del botón: solo devolviendo 1 desde el gancho se
//! "traga" el clic central, y si no se traga, la ventana de debajo empieza el autodesplazamiento y
//! arrastra la página mientras apuntas.
//!
//! Los ganchos se ejecutan en el hilo que los instaló (uno propio, con su bucle de mensajes) y
//! tienen que volver enseguida, así que no deciden nada: actualizan `ESTADO` y avisan a la ventana
//! con `PostMessageW`. La ventana hace el trabajo en el hilo principal.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread::JoinHandle;
use windows::Win32::System::Threading::{
    GetCurrentThread, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, SetThreadPriority,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, THREAD_PRIORITY_HIGHEST,
};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use crate::config::Boton;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Mensajes que el gancho envía a la ventana principal.
pub const WM_ORBYT_DOWN: u32 = WM_APP + 1;
pub const WM_ORBYT_TAP: u32 = WM_APP + 2;
pub const WM_ORBYT_RELEASE: u32 = WM_APP + 3;
pub const WM_ORBYT_CANCEL: u32 = WM_APP + 4;
/// En modo clic, un clic con la rueda abierta. `lparam` es 0 si es el izquierdo, 1 el derecho.
pub const WM_ORBYT_CLIC: u32 = WM_APP + 5;

/// Sin nada en curso.
pub const INACTIVO: u8 = 0;
/// Botón pulsado y tragado; aún no se sabe si es un clic o una pulsación larga.
pub const PENDIENTE: u8 = 1;
/// La rueda está abierta.
pub const ABIERTA: u8 = 2;
/// Se cerró con el botón aún pulsado: falta tragar su liberación para que no llegue suelta.
pub const CANCELADA: u8 = 3;

pub static ESTADO: AtomicU8 = AtomicU8::new(INACTIVO);
/// Modo clic: la rueda sigue abierta al soltar el botón y se elige con clics.
static MODO_CLIC: AtomicBool = AtomicBool::new(false);
/// El botón que abre la rueda está pulsado ahora mismo.
static BOTON_ABAJO: AtomicBool = AtomicBool::new(false);
/// Se tragó la pulsación de un clic: su liberación también hay que tragarla.
static TRAGAR_IZQ: AtomicBool = AtomicBool::new(false);
static TRAGAR_DER: AtomicBool = AtomicBool::new(false);

/// Con una ventana a pantalla completa delante, el botón pasa sin tocar.
static PAUSAR_PANTALLA_COMPLETA: AtomicBool = AtomicBool::new(true);
/// Ejecutables (en minúsculas) en los que el botón pasa sin tocar.
static EXCLUIDAS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Última ventana activa consultada y si su programa está excluido: abrir el proceso para leer
/// su nombre solo hace falta cuando cambia la ventana.
static ULTIMA_EXCLUIDA: Mutex<(isize, bool)> = Mutex::new((0, false));

pub fn poner_exclusiones(pausar_pantalla_completa: bool, excluidas: &[String]) {
    PAUSAR_PANTALLA_COMPLETA.store(pausar_pantalla_completa, Ordering::Relaxed);
    let lista: Vec<String> = excluidas.iter().map(|e| e.trim().to_lowercase()).filter(|e| !e.is_empty()).collect();
    *EXCLUIDAS.lock().unwrap_or_else(|e| e.into_inner()) = lista;
    *ULTIMA_EXCLUIDA.lock().unwrap_or_else(|e| e.into_inner()) = (0, false);
}

/// ¿Hay que dejar pasar el botón a la ventana activa sin que Orbyt haga nada? Se decide en el
/// gancho, al pulsar, así que solo hace llamadas que responden al instante.
fn cedido() -> bool {
    let activa = unsafe { GetForegroundWindow() };
    if activa.is_invalid() {
        return false;
    }
    (PAUSAR_PANTALLA_COMPLETA.load(Ordering::Relaxed) && pantalla_completa(activa)) || excluida(activa)
}

/// La ventana cubre todo su monitor y no tiene barra de título: un juego (exclusivo o "sin
/// bordes") o un vídeo a pantalla completa. Una ventana maximizada conserva su barra y no cuenta;
/// el escritorio tampoco.
fn pantalla_completa(ventana: HWND) -> bool {
    unsafe {
        let mut clase = [0u16; 64];
        let n = GetClassNameW(ventana, &mut clase) as usize;
        let clase = String::from_utf16_lossy(&clase[..n]);
        if matches!(clase.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
            return false;
        }
        let mut rect = RECT::default();
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetWindowRect(ventana, &mut rect).is_err()
            || !GetMonitorInfoW(MonitorFromWindow(ventana, MONITOR_DEFAULTTONEAREST), &mut info).as_bool()
        {
            return false;
        }
        cubre_monitor(&rect, &info.rcMonitor, GetWindowLongW(ventana, GWL_STYLE) as u32)
    }
}

fn cubre_monitor(ventana: &RECT, monitor: &RECT, estilo: u32) -> bool {
    estilo & WS_CAPTION.0 != WS_CAPTION.0
        && ventana.left <= monitor.left
        && ventana.top <= monitor.top
        && ventana.right >= monitor.right
        && ventana.bottom >= monitor.bottom
}

fn excluida(ventana: HWND) -> bool {
    let lista = EXCLUIDAS.lock().unwrap_or_else(|e| e.into_inner());
    if lista.is_empty() {
        return false;
    }
    let mut ultima = ULTIMA_EXCLUIDA.lock().unwrap_or_else(|e| e.into_inner());
    if ultima.0 == ventana.0 as isize {
        return ultima.1;
    }
    let excluida = ejecutable_de(ventana).is_some_and(|exe| lista.contains(&exe));
    *ultima = (ventana.0 as isize, excluida);
    excluida
}

/// Nombre del ejecutable de una ventana, en minúsculas (`blender.exe`).
fn ejecutable_de(ventana: HWND) -> Option<String> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(ventana, Some(&mut pid));
        let proceso = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut ruta = [0u16; 1024];
        let mut largo = ruta.len() as u32;
        let leido = QueryFullProcessImageNameW(proceso, PROCESS_NAME_WIN32, PWSTR(ruta.as_mut_ptr()), &mut largo);
        let _ = CloseHandle(proceso);
        leido.ok()?;
        let ruta = String::from_utf16_lossy(&ruta[..largo as usize]);
        ruta.rsplit(['\\', '/']).next().map(str::to_lowercase)
    }
}

pub fn poner_modo_clic(clic: bool) {
    MODO_CLIC.store(clic, Ordering::Relaxed);
}

/// La rueda se cerró: si el botón sigue pulsado, falta tragar su liberación.
pub fn terminar() {
    let estado = if BOTON_ABAJO.load(Ordering::SeqCst) { CANCELADA } else { INACTIVO };
    ESTADO.store(estado, Ordering::SeqCst);
}
static VENTANA: AtomicIsize = AtomicIsize::new(0);

/// Identificador del hilo de los ganchos; 0 si no está en marcha.
static HILO_ID: AtomicU32 = AtomicU32::new(0);
static HILO: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);
/// Mensaje de hilo: poner (`wparam` = 1) o quitar (0) el gancho del teclado.
const WM_GANCHO_TECLADO: u32 = WM_APP + 60;

/// Instala el gancho del ratón en un hilo propio, que no hace nada más que atenderlo.
///
/// Un gancho de bajo nivel se ejecuta en el hilo que lo instaló, y todo el ratón del sistema
/// espera su respuesta. Si compartiera hilo con la ventana de ajustes y la extracción de íconos,
/// cualquier trabajo largo ahí trabaría el puntero, y pasado el tiempo límite Windows quita el
/// gancho sin avisar. Aquí el gancho solo toca atómicos y publica mensajes, así que siempre
/// responde enseguida.
///
/// El del teclado no se instala todavía: solo hace falta con la rueda abierta (para Escape), y
/// puesto todo el tiempo haría pasar cada tecla del sistema por este proceso.
pub fn instalar(ventana: HWND) -> windows::core::Result<()> {
    VENTANA.store(ventana.0 as isize, Ordering::SeqCst);
    let (listo, instalado) = mpsc::channel();
    let hilo = std::thread::Builder::new()
        .name("ganchos".into())
        .spawn(move || unsafe {
            // Crea la cola de mensajes del hilo antes de publicar su identificador.
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE);
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            let raton_h = match SetWindowsHookExW(WH_MOUSE_LL, Some(raton), None, 0) {
                Ok(h) => h,
                Err(e) => {
                    let _ = listo.send(Err(e));
                    return;
                }
            };
            HILO_ID.store(GetCurrentThreadId(), Ordering::SeqCst);
            let _ = listo.send(Ok(()));
            let mut teclado_h: Option<HHOOK> = None;
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_GANCHO_TECLADO {
                    if msg.wParam.0 != 0 {
                        if teclado_h.is_none() {
                            teclado_h = SetWindowsHookExW(WH_KEYBOARD_LL, Some(teclado), None, 0).ok();
                        }
                    } else if let Some(h) = teclado_h.take() {
                        let _ = UnhookWindowsHookEx(h);
                    }
                } else {
                    DispatchMessageW(&msg);
                }
            }
            if let Some(h) = teclado_h {
                let _ = UnhookWindowsHookEx(h);
            }
            let _ = UnhookWindowsHookEx(raton_h);
        })
        .map_err(|_| windows::core::Error::from_thread())?;
    match instalado.recv() {
        Ok(Ok(())) => {
            *HILO.lock().unwrap_or_else(|e| e.into_inner()) = Some(hilo);
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(windows::core::Error::from_thread()),
    }
}

/// Pone o quita el gancho del teclado, al abrir y al cerrar la rueda. Lo hace el hilo de los
/// ganchos: un gancho se atiende en el hilo que lo instala.
pub fn poner_teclado(activo: bool) {
    let id = HILO_ID.load(Ordering::SeqCst);
    if id != 0 {
        unsafe {
            let _ = PostThreadMessageW(id, WM_GANCHO_TECLADO, WPARAM(activo as usize), LPARAM(0));
        }
    }
}

/// Quita los ganchos y espera a que su hilo termine. Se puede llamar más de una vez.
pub fn quitar() {
    let id = HILO_ID.swap(0, Ordering::SeqCst);
    if id != 0 {
        unsafe {
            let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
    if let Some(hilo) = HILO.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = hilo.join();
    }
}

fn avisar(msg: u32, lparam: isize) {
    let hwnd = HWND(VENTANA.load(Ordering::Relaxed) as *mut _);
    unsafe {
        let _ = PostMessageW(Some(hwnd), msg, WPARAM(0), LPARAM(lparam));
    }
}

/// Empaqueta un punto de pantalla (puede ser negativo con varios monitores) en un `LPARAM`.
pub fn empaquetar(x: i32, y: i32) -> isize {
    ((x as u16 as u32) | ((y as u16 as u32) << 16)) as isize
}

pub fn desempaquetar(l: isize) -> (i32, i32) {
    ((l as u32 & 0xFFFF) as u16 as i16 as i32, ((l as u32 >> 16) & 0xFFFF) as u16 as i16 as i32)
}

/// Botón que abre la rueda: 0 central, 1 lateral "atrás", 2 lateral "adelante".
static BOTON: AtomicU8 = AtomicU8::new(0);

pub fn poner_boton(boton: Boton) {
    let v = match boton {
        Boton::Central => 0,
        Boton::Atras => 1,
        Boton::Adelante => 2,
    };
    BOTON.store(v, Ordering::Relaxed);
}

pub fn boton() -> Boton {
    match BOTON.load(Ordering::Relaxed) {
        1 => Boton::Atras,
        2 => Boton::Adelante,
        _ => Boton::Central,
    }
}

/// ¿Es este mensaje del ratón el botón configurado? Devuelve `Some(true)` al pulsarlo,
/// `Some(false)` al soltarlo y `None` si es otro botón.
fn es_el_boton(msg: u32, datos: u32) -> Option<bool> {
    // En los botones laterales, la palabra alta de `mouseData` dice cuál es (1 o 2).
    let lateral = (datos >> 16) as u16;
    match (BOTON.load(Ordering::Relaxed), msg) {
        (0, WM_MBUTTONDOWN) => Some(true),
        (0, WM_MBUTTONUP) => Some(false),
        (1, WM_XBUTTONDOWN) if lateral == XBUTTON1 => Some(true),
        (1, WM_XBUTTONUP) if lateral == XBUTTON1 => Some(false),
        (2, WM_XBUTTONDOWN) if lateral == XBUTTON2 => Some(true),
        (2, WM_XBUTTONUP) if lateral == XBUTTON2 => Some(false),
        _ => None,
    }
}

unsafe extern "system" fn raton(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Casi todo lo que llega es movimiento: sale por aquí sin mirar nada más. Con la rueda
    // abierta, además, pide un fotograma si el pulso está parado.
    if code == HC_ACTION as i32 && wparam.0 as u32 == WM_MOUSEMOVE {
        if ESTADO.load(Ordering::Relaxed) == ABIERTA {
            crate::ritmo::al_mover();
        }
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    if code == HC_ACTION as i32 {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        // Los eventos inyectados (incluido el clic que Orbyt reenvía) pasan sin tocar.
        let inyectado = info.flags & LLMHF_INJECTED != 0;
        if !inyectado {
            let msg = wparam.0 as u32;
            let clic = MODO_CLIC.load(Ordering::Relaxed);
            match es_el_boton(msg, info.mouseData) {
                Some(true) => {
                    // En un juego a pantalla completa o en un programa excluido el botón no es
                    // para Orbyt: pasa tal cual, y su liberación también (el estado no cambia).
                    if ESTADO.load(Ordering::SeqCst) == INACTIVO && cedido() {
                        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                    }
                    BOTON_ABAJO.store(true, Ordering::SeqCst);
                    if ESTADO
                        .compare_exchange(INACTIVO, PENDIENTE, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        avisar(WM_ORBYT_DOWN, empaquetar(info.pt.x, info.pt.y));
                        return LRESULT(1);
                    }
                    // Modo clic, con la rueda abierta: volver a pulsar el botón la cierra.
                    if ESTADO
                        .compare_exchange(ABIERTA, CANCELADA, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        avisar(WM_ORBYT_CANCEL, 0);
                        return LRESULT(1);
                    }
                }
                Some(false) => {
                    BOTON_ABAJO.store(false, Ordering::SeqCst);
                    match ESTADO.load(Ordering::SeqCst) {
                        PENDIENTE => {
                            ESTADO.store(INACTIVO, Ordering::SeqCst);
                            avisar(WM_ORBYT_TAP, 0);
                            return LRESULT(1);
                        }
                        // En modo clic, soltar no elige nada: la rueda queda abierta.
                        ABIERTA if clic => return LRESULT(1),
                        ABIERTA => {
                            ESTADO.store(INACTIVO, Ordering::SeqCst);
                            avisar(WM_ORBYT_RELEASE, 0);
                            return LRESULT(1);
                        }
                        CANCELADA => {
                            ESTADO.store(INACTIVO, Ordering::SeqCst);
                            return LRESULT(1);
                        }
                        _ => {}
                    }
                }
                None => {
                    // Modo clic, con la rueda abierta: los clics son para la rueda y no llegan a
                    // la ventana de debajo (ni su pulsación ni su liberación).
                    if clic && ESTADO.load(Ordering::SeqCst) == ABIERTA {
                        match msg {
                            WM_LBUTTONDOWN => {
                                TRAGAR_IZQ.store(true, Ordering::SeqCst);
                                avisar(WM_ORBYT_CLIC, 0);
                                return LRESULT(1);
                            }
                            WM_RBUTTONDOWN => {
                                TRAGAR_DER.store(true, Ordering::SeqCst);
                                avisar(WM_ORBYT_CLIC, 1);
                                return LRESULT(1);
                            }
                            _ => {}
                        }
                    }
                    let tragar = match msg {
                        WM_LBUTTONUP => TRAGAR_IZQ.swap(false, Ordering::SeqCst),
                        WM_RBUTTONUP => TRAGAR_DER.swap(false, Ordering::SeqCst),
                        _ => false,
                    };
                    if tragar {
                        return LRESULT(1);
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn teclado(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && ESTADO.load(Ordering::Relaxed) == ABIERTA {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if info.vkCode == VK_ESCAPE.0 as u32 {
            let pulsada = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            if pulsada && ESTADO.load(Ordering::SeqCst) == ABIERTA {
                // Con el botón aún pulsado queda CANCELADA (falta tragar su liberación); en modo
                // clic, ya suelto, vuelve directamente a INACTIVO.
                terminar();
                avisar(WM_ORBYT_CANCEL, 0);
            }
            // Escape no llega a la ventana de debajo mientras la rueda está abierta.
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_cubrir_el_monitor_sin_barra_de_titulo_es_pantalla_completa() {
        let monitor = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        let (popup, normal) = (WS_POPUP.0, WS_OVERLAPPEDWINDOW.0);
        assert!(cubre_monitor(&monitor, &monitor, popup));
        // Maximizada: sobresale unos píxeles pero conserva la barra de título.
        assert!(!cubre_monitor(&RECT { left: -8, top: -8, right: 1928, bottom: 1088 }, &monitor, normal));
        assert!(!cubre_monitor(&RECT { left: 0, top: 0, right: 1920, bottom: 1040 }, &monitor, popup));
    }

    #[test]
    fn empaquetar_conserva_coordenadas_negativas() {
        for (x, y) in [(0, 0), (1920, 1080), (-1280, 200), (-5, -3000)] {
            assert_eq!(desempaquetar(empaquetar(x, y)), (x, y));
        }
    }
}
