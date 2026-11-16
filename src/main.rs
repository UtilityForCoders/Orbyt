//! Orbyt: lanzador radial para Windows.
//!
//! Mantén pulsado el botón central del ratón (o uno lateral, según los ajustes) en cualquier
//! sitio, apunta hacia un acceso y suelta. Un clic corto se reenvía tal cual, así que abrir
//! enlaces en otra pestaña sigue funcionando.
//!
//! La ventana de la rueda, los ajustes y el ícono de la bandeja comparten el bucle de mensajes del
//! hilo principal. Los ganchos de entrada van en un hilo propio (ver `hook.rs`) para que nada de
//! lo que pase aquí pueda trabar el ratón del sistema.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Un brazo de `match` que nombra una constante que no está importada no es un error: Rust lo lee
// como una variable nueva que atrapa cualquier valor y deja muertos los brazos de debajo. En un
// procedimiento de ventana eso es una ventana que pinta bien y no responde a nada (pasó con
// `WM_MOUSELEAVE`, que vive en `UI::Controls`). Así deja de compilar en lugar de ser un aviso.
#![deny(unreachable_patterns)]

mod actualizacion;
mod apps;
mod atajo;
mod autostart;
mod config;
mod hook;
mod icons;
mod launch;
mod lucide;
mod lucide_datos;
mod menu;
mod render;
mod ritmo;
mod settings;
mod tray;
mod ui;
mod ventanas;
mod web;
mod wheel;

use config::{AbrirEn, Boton, Config, Modo};
use hook::*;
use render::{Baldosa, Escena, Renderer};
use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::time::Instant;
use windows::core::{w, Result};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const TIMER_MANTENER: usize = 1;
const TIMER_DEMO: usize = 3;
/// Duración de la animación de apertura.
const APERTURA_MS: f32 = 140.0;
/// Identificador del atajo de teclado global (`RegisterHotKey`).
const ID_ATAJO: i32 = 1;
/// Cuánto puede moverse el puntero (en píxeles) sin que el ratón le quite el mando al teclado:
/// con el botón mantenido, la mano nunca está quieta del todo.
const HOLGURA_TECLADO: i32 = 6;

struct Rueda {
    /// Esquina superior izquierda del monitor, en coordenadas de pantalla.
    origen: POINT,
    ancho: i32,
    alto: i32,
    /// Centro de la rueda relativo a la ventana.
    centro: (f32, f32),
    escala: f32,
    seleccion: Option<usize>,
    inicio: Instant,
    progreso: f32,
    /// Progreso visible antes de abrir un grupo por permanencia.
    progreso_grupo: f32,
    /// Grupo abierto; `None` en la rueda principal.
    nivel: Option<usize>,
    /// Grupo al que se está apuntando en la raíz, y desde cuándo (para abrirlo al rato).
    apuntado: Option<(usize, Instant)>,
    /// Dentro de un grupo, desde cuándo el puntero está en el círculo central (para volver).
    en_centro: Option<Instant>,
    /// Se está usando el teclado: dónde estaba el puntero en la última tecla y qué baldosa
    /// quedó elegida con ella. Mientras el puntero siga ahí manda el teclado, y las esperas del
    /// ratón (abrir un grupo, volver desde el centro) no corren.
    teclado: Option<(POINT, Option<usize>)>,
}

struct App {
    hwnd: HWND,
    /// Capa negra de opacidad constante; evita incluir todo el monitor en el bitmap de la rueda.
    fondo: HWND,
    cfg: Config,
    /// Una baldosa por grupo, en el mismo orden que `cfg.grupos`.
    raiz: Vec<Baldosa>,
    /// Grupos que muestra la rueda principal (posiciones en `cfg.grupos`): los que no son de un
    /// programa concreto y los del programa que había delante al abrirla.
    vista: Vec<usize>,
    /// Las baldosas de `vista`, en ese orden.
    raiz_vista: Vec<Baldosa>,
    /// Baldosas de cada grupo, en el mismo orden que `cfg.grupos`.
    grupos: Vec<Vec<Baldosa>>,
    renderer: Renderer,
    pulsado_en: POINT,
    rueda: Option<Rueda>,
    /// Fecha y tamaño de `config.json` en la última lectura.
    firma: Option<(std::time::SystemTime, u64)>,
    /// Atajo de teclado tal como se pidió registrar la última vez, y su tecla si se consiguió.
    atajo: Option<String>,
    tecla_atajo: u32,
    /// Error de `config.json` del que ya se avisó, para no repetir el aviso en cada pulsación.
    error_config: Option<String>,
}

fn firma_config() -> Option<(std::time::SystemTime, u64)> {
    let m = std::fs::metadata(config::ruta()).ok()?;
    Some((m.modified().ok()?, m.len()))
}

impl App {
    /// Relee `config.json` solo si cambió desde la última vez. Se llama en cada pulsación del
    /// botón (también en los clics cortos que se reenvían), así que leer y parsear el archivo y
    /// rehacer las baldosas cada vez era trabajo tirado.
    fn recargar_si_cambio(&mut self) {
        let firma = firma_config();
        if firma.is_none() || firma != self.firma {
            self.recargar();
        }
    }

    /// Relee `config.json` y prepara los íconos. Los ya extraídos salen de la caché, así que
    /// después de la primera vez esto es casi gratis.
    fn recargar(&mut self) {
        let (cfg, error) = config::leer();
        self.cfg = cfg;
        if error != self.error_config {
            if let Some(e) = &error {
                tray::avisar(
                    "config.json tiene un error",
                    &format!("Se usan los valores por defecto; hay una copia en config.invalido.json. {e}"),
                );
            }
            self.error_config = error;
        }
        // Después de cargar: si el archivo no existía, `leer` acaba de crearlo.
        self.firma = firma_config();
        self.raiz = render::baldosas_raiz(&self.cfg);
        self.grupos = self.cfg.grupos.iter().map(render::baldosas_grupo).collect();
        self.rehacer_vista();
        hook::poner_boton(self.cfg.boton);
        hook::poner_modo_clic(self.cfg.modo == Modo::Clic);
        hook::poner_exclusiones(self.cfg.pausar_pantalla_completa, &self.cfg.excluidas);
        self.aplicar_atajo();
    }

    /// Elige los grupos de la rueda principal según el programa que hay delante.
    fn elegir_vista(&mut self) {
        let exe = ventanas::ejecutable_activo();
        self.vista = (0..self.cfg.grupos.len())
            .filter(|&g| self.cfg.grupos[g].visible_en(exe.as_deref()))
            .collect();
        self.rehacer_vista();
    }

    fn rehacer_vista(&mut self) {
        self.vista.retain(|&g| g < self.raiz.len());
        self.raiz_vista = self.vista.iter().map(|&g| self.raiz[g].clone()).collect();
    }

    /// Registra en Windows el atajo de teclado de la configuración, si cambió.
    fn aplicar_atajo(&mut self) {
        if self.cfg.atajo == self.atajo {
            return;
        }
        unsafe {
            let _ = UnregisterHotKey(Some(self.hwnd), ID_ATAJO);
        }
        // Se anota aunque falle: así el aviso sale una vez y no en cada recarga.
        self.atajo = self.cfg.atajo.clone();
        self.tecla_atajo = 0;
        let Some(texto) = self.atajo.clone() else { return };
        let registrado = atajo::leer(&texto).filter(|&(mods, vk)| unsafe {
            RegisterHotKey(Some(self.hwnd), ID_ATAJO, HOT_KEY_MODIFIERS(mods) | MOD_NOREPEAT, vk).is_ok()
        });
        match registrado {
            Some((_, vk)) => self.tecla_atajo = vk,
            None => tray::avisar(
                "El atajo de teclado no está disponible",
                &format!("«{texto}» no es válido o ya lo usa otro programa. Elige otro en Configuración → Activación."),
            ),
        }
    }

    /// Baldosas del nivel que muestra la rueda.
    fn baldosas(&self, nivel: Option<usize>) -> &[Baldosa] {
        match nivel.and_then(|g| self.grupos.get(g)) {
            Some(b) => b,
            None => &self.raiz_vista,
        }
    }

    /// Acceso que abre la baldosa `i` del nivel; `None` en la rueda principal, que solo tiene
    /// grupos.
    fn acceso(&self, nivel: Option<usize>, i: usize) -> Option<config::Acceso> {
        self.cfg.grupos.get(nivel?)?.accesos.get(i).cloned()
    }
}

/// Abre el acceso elegido en la rueda, si lo hay.
fn lanzar(elegido: Option<config::Acceso>) {
    if let Some(acceso) = elegido {
        let enfocar = con_app(|app| app.cfg.enfocar_abiertas).unwrap_or(true);
        launch::abrir(&acceso, enfocar);
    }
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Si el estado ya está prestado (un mensaje que llega mientras se atiende otro), no se hace
/// nada en lugar de entrar en pánico.
fn con_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| match a.try_borrow_mut() {
        Ok(mut g) => g.as_mut().map(f),
        Err(_) => None,
    })
}

/// Deja constancia de un fallo interno en `%APPDATA%\Orbyt\fallo.log`. La app no tiene consola:
/// sin esto, un fallo solo se ve como que Orbyt desapareció de la bandeja.
fn registrar_fallos() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let ruta = config::ruta().with_file_name("fallo.log");
        if let Ok(mut archivo) = std::fs::OpenOptions::new().create(true).append(true).open(ruta) {
            let cuando = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let _ = writeln!(archivo, "[{cuando}] Orbyt {}: {info}", env!("CARGO_PKG_VERSION"));
        }
    }));
}

fn main() -> Result<()> {
    registrar_fallos();
    unsafe {
        // Una sola instancia: dos ganchos peleándose por el mismo botón no sirven para nada.
        let _mutex = CreateMutexW(None, true, w!("Local\\OrbytLanzadorRadial"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
        // Coordenadas físicas en todos los monitores: el gancho y la ventana hablan el mismo idioma.
        // (El manifiesto ya lo pide; esto cubre ejecutar sin él.)
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // COM de un solo hilo: lo piden el shell (íconos, ShellExecute) y los diálogos.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let instancia: HINSTANCE = GetModuleHandleW(None)?.into();
        let clase = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instancia,
            lpszClassName: w!("OrbytRueda"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        RegisterClassW(&clase);
        let clase_fondo = WNDCLASSW {
            lpfnWndProc: Some(wndproc_fondo),
            hInstance: instancia,
            lpszClassName: w!("OrbytFondo"),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ..Default::default()
        };
        RegisterClassW(&clase_fondo);

        // El oscurecimiento usa una ventana sólida con alfa constante. El compositor puede
        // mantenerla sin copiar un bitmap de pantalla completa en cada fotograma de la rueda.
        let fondo = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("OrbytFondo"),
            w!("Orbyt"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instancia),
            None,
        )?;
        SetLayeredWindowAttributes(fondo, COLORREF(0), 72, LWA_ALPHA)?;

        // La rueda nunca toma el foco ni recibe clics: la ventana en la que estabas sigue siendo
        // la activa y el teclado llega por el gancho.
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("OrbytRueda"),
            w!("Orbyt"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instancia),
            None,
        )?;

        let mut app = App {
            hwnd,
            fondo,
            cfg: Config::default(),
            raiz: Vec::new(),
            vista: Vec::new(),
            raiz_vista: Vec::new(),
            grupos: Vec::new(),
            renderer: Renderer::nuevo().expect("no se pudo iniciar Direct2D"),
            pulsado_en: POINT::default(),
            rueda: None,
            firma: None,
            atajo: None,
            tecla_atajo: 0,
            error_config: None,
        };
        // El ícono va antes de leer la configuración: es por donde se avisa si está mal escrita.
        tray::agregar(hwnd);
        app.recargar();
        APP.with(|a| *a.borrow_mut() = Some(app));

        hook::instalar(hwnd)?;
        web::poner_aviso(hwnd);

        // `orbyt --demo`: abre la rueda bajo el puntero durante unos segundos sin pulsar nada.
        // Sirve para revisar el dibujo (el gancho ignora los clics simulados con SendInput).
        if std::env::args().any(|a| a == "--demo") {
            ESTADO.store(ABIERTA, Ordering::SeqCst);
            con_app(|app| {
                let _ = GetCursorPos(&mut app.pulsado_en);
                abrir(app);
            });
            SetTimer(Some(hwnd), TIMER_DEMO, 4000, None);
        }
        // `orbyt --ajustes`: abre la ventana de configuración al arrancar.
        if std::env::args().any(|a| a == "--ajustes") {
            settings::abrir(hwnd);
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        tray::quitar(hwnd);
        hook::quitar();
        // El estado (Direct2D, íconos) se libera aquí y no en los destructores de fin de proceso.
        APP.with(|a| a.borrow_mut().take());
    }
    Ok(())
}

unsafe extern "system" fn wndproc_fondo(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ORBYT_DOWN => {
            let (x, y) = desempaquetar(lparam.0);
            con_app(|app| {
                app.pulsado_en = POINT { x, y };
                // El temporizador arranca antes de releer: el tiempo de leer el archivo (y algún
                // ícono nuevo) corre a la vez que la espera, no después.
                unsafe { SetTimer(Some(hwnd), TIMER_MANTENER, app.cfg.retardo_ms, None) };
                // Se comprueba en cada pulsación: los cambios en config.json valen sin reiniciar.
                app.recargar_si_cambio();
            });
        }
        web::WM_ORBYT_FAVICON => {
            // Llegó un favicon: se releen los íconos web en la rueda y en los ajustes.
            icons::olvidar_webs();
            con_app(App::recargar);
            settings::refrescar();
        }
        settings::WM_ORBYT_CONFIG => {
            // La ventana de configuración guardó: se precargan los íconos nuevos ya, para que
            // la próxima rueda no tenga que esperar por ellos.
            con_app(App::recargar);
        }
        WM_ORBYT_TAP => {
            unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_MANTENER);
            }
            reenviar_clic(hook::boton());
        }
        WM_ORBYT_RELEASE => {
            let elegido = con_app(|app| {
                actualizar(app);
                // Soltar sobre un grupo que aún no se abrió no abre nada.
                let elegido = app.rueda.as_ref().and_then(|r| app.acceso(r.nivel, r.seleccion?));
                cerrar(app);
                elegido
            })
            .flatten();
            lanzar(elegido);
        }
        WM_ORBYT_CANCEL => {
            con_app(cerrar);
        }
        ritmo::WM_ORBYT_FOTOGRAMA => {
            ritmo::atendido();
            con_app(actualizar);
        }
        WM_ORBYT_CLIC => {
            let elegido = con_app(|app| clic(app, lparam.0 == 1)).flatten();
            lanzar(elegido);
        }
        WM_ORBYT_TECLA => {
            let elegido = con_app(|app| tecla(app, lparam.0 as u32)).flatten();
            lanzar(elegido);
        }
        WM_HOTKEY if wparam.0 as i32 == ID_ATAJO => {
            // El atajo abre la rueda sin botón mantenido: queda abierta y se elige con clic o con
            // el teclado. Pulsarlo otra vez la cierra. En un juego a pantalla completa o en un
            // programa excluido no hace nada, igual que el botón.
            match ESTADO.compare_exchange(INACTIVO, ABIERTA, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) if hook::cedido() => ESTADO.store(INACTIVO, Ordering::SeqCst),
                Ok(_) => {
                    con_app(|app| {
                        hook::poner_por_teclado(true, app.tecla_atajo);
                        unsafe {
                            let _ = GetCursorPos(&mut app.pulsado_en);
                        }
                        app.recargar_si_cambio();
                        abrir(app);
                    });
                }
                Err(ABIERTA) => {
                    hook::terminar();
                    con_app(cerrar);
                }
                Err(_) => {}
            }
        }
        WM_TIMER => match wparam.0 {
            TIMER_MANTENER => {
                unsafe {
                    let _ = KillTimer(Some(hwnd), TIMER_MANTENER);
                }
                // Si el botón ya se soltó, el gancho dejó el estado en INACTIVO y esto no hace nada.
                if ESTADO
                    .compare_exchange(PENDIENTE, ABIERTA, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    con_app(abrir);
                }
            }
            TIMER_DEMO => unsafe {
                let _ = KillTimer(Some(hwnd), TIMER_DEMO);
                if ESTADO
                    .compare_exchange(ABIERTA, INACTIVO, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    con_app(cerrar);
                }
            },
            _ => {}
        },
        tray::WM_ORBYT_TRAY => {
            let evento = (lparam.0 as u32) & 0xFFFF;
            if evento == WM_LBUTTONUP {
                settings::abrir(hwnd);
            } else if evento == WM_RBUTTONUP {
                menu::mostrar(hwnd);
            }
        }
        menu::WM_ORBYT_MENU => match wparam.0 {
            menu::CMD_CONFIGURAR => settings::abrir(hwnd),
            menu::CMD_SALIR => unsafe {
                let _ = DestroyWindow(hwnd);
            },
            _ => {}
        },
        WM_DESTROY => unsafe {
            // Lo primero, soltar el ratón y el teclado del sistema; después, cerrar los ajustes
            // con el proceso aún vivo (dejar que se destruyan al terminar el proceso tardaba
            // varios segundos, y con los ganchos puestos el puntero se congelaba).
            hook::quitar();
            let _ = UnregisterHotKey(Some(hwnd), ID_ATAJO);
            settings::cerrar();
            PostQuitMessage(0)
        },
        m if tray::es_barra_creada(m) => tray::agregar(hwnd),
        _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
    LRESULT(0)
}

/// Un clic corto no era para Orbyt: se devuelve al sistema como si nada (el gancho deja pasar
/// los eventos inyectados).
fn reenviar_clic(boton: Boton) {
    let evento = |flags, datos: u16| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                mouseData: datos as u32,
                ..Default::default()
            },
        },
    };
    let entradas = match boton {
        Boton::Central => [evento(MOUSEEVENTF_MIDDLEDOWN, 0), evento(MOUSEEVENTF_MIDDLEUP, 0)],
        Boton::Atras => [evento(MOUSEEVENTF_XDOWN, XBUTTON1), evento(MOUSEEVENTF_XUP, XBUTTON1)],
        Boton::Adelante => [evento(MOUSEEVENTF_XDOWN, XBUTTON2), evento(MOUSEEVENTF_XUP, XBUTTON2)],
    };
    unsafe {
        SendInput(&entradas, std::mem::size_of::<INPUT>() as i32);
    }
}

fn abrir(app: &mut App) {
    let p = app.pulsado_en;
    // La rueda no toma el foco: el programa de delante sigue siendo el que había al pulsar.
    app.elegir_vista();
    unsafe {
        let monitor = MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        // El tamaño elegido en los ajustes se aplica como una escala más.
        let escala = dpi_x as f32 / 96.0 * app.cfg.tamano;

        let m = info.rcMonitor;
        let (ancho_monitor, alto_monitor) = (m.right - m.left, m.bottom - m.top);
        // El bitmap contiene solo la rueda y sus etiquetas, no el monitor completo. En 4K esto
        // reduce cada actualización de unos 32 MiB a alrededor de 1–2 MiB.
        let mayor = app
            .vista
            .iter()
            .filter_map(|&g| app.grupos.get(g))
            .map(Vec::len)
            .chain([app.raiz_vista.len()])
            .max()
            .unwrap_or(0);
        let margen = wheel::alcance(mayor) * escala;
        let ancho = ((2.0 * margen).ceil() as i32).min(ancho_monitor);
        let alto = ((2.0 * margen).ceil() as i32).min(alto_monitor);
        let limitar = |v: f32, minimo: i32, maximo: i32, lado: i32| {
            let mitad = lado as f32 / 2.0;
            if maximo - minimo <= lado {
                (minimo + maximo) as f32 / 2.0
            } else {
                v.clamp(minimo as f32 + mitad, maximo as f32 - mitad)
            }
        };
        let centro_pantalla = match app.cfg.abrir_en {
            AbrirEn::Centro => ((m.left + m.right) as f32 / 2.0, (m.top + m.bottom) as f32 / 2.0),
            AbrirEn::Cursor => (
                limitar(p.x as f32, m.left, m.right, ancho),
                limitar(p.y as f32, m.top, m.bottom, alto),
            ),
        };
        let origen = POINT {
            x: (centro_pantalla.0 - ancho as f32 / 2.0).round() as i32,
            y: (centro_pantalla.1 - alto as f32 / 2.0).round() as i32,
        };
        let centro = (centro_pantalla.0 - origen.x as f32, centro_pantalla.1 - origen.y as f32);

        app.rueda = Some(Rueda {
            origen,
            ancho,
            alto,
            centro,
            escala,
            seleccion: None,
            inicio: Instant::now(),
            progreso: 0.0,
            progreso_grupo: 0.0,
            nivel: None,
            apuntado: None,
            en_centro: None,
            teclado: None,
        });
        if app.cfg.oscurecer {
            let _ = SetWindowPos(
                app.fondo,
                Some(HWND_TOPMOST),
                m.left,
                m.top,
                ancho_monitor,
                alto_monitor,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        } else {
            let _ = ShowWindow(app.fondo, SW_HIDE);
        }
        // El primer fotograma se pinta antes de mostrar la ventana, para que nunca se vea vacía.
        pintar(app);
        let _ = ShowWindow(app.hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            app.hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        hook::poner_teclado(true);
        ritmo::empezar(app.hwnd);
    }
}

fn cerrar(app: &mut App) {
    hook::poner_teclado(false);
    hook::poner_por_teclado(false, 0);
    unsafe {
        ritmo::parar();
        let _ = ShowWindow(app.hwnd, SW_HIDE);
        let _ = ShowWindow(app.fondo, SW_HIDE);
    }
    app.rueda = None;
}

/// Muestra otro nivel: un grupo o, con `None`, la rueda principal.
fn navegar(app: &mut App, nivel: Option<usize>) {
    let Some(r) = app.rueda.as_mut() else { return };
    r.nivel = nivel;
    r.apuntado = None;
    r.en_centro = None;
    r.teclado = None;
    r.seleccion = None;
    r.progreso_grupo = 0.0;
    // La animación de apertura se repite: las baldosas del nuevo nivel salen del centro.
    r.inicio = Instant::now();
    r.progreso = 0.0;
    pintar(app);
    // La animación vuelve a empezar: hace falta el pulso aunque estuviera parado.
    ritmo::empezar(app.hwnd);
}

/// Abre la baldosa `i` del nivel actual: entra en el grupo o devuelve el acceso a lanzar (y cierra
/// la rueda). `por_tecla` deja el mando en el teclado después de entrar en un grupo.
fn activar(app: &mut App, i: usize, por_tecla: bool) -> Option<config::Acceso> {
    let nivel = app.rueda.as_ref()?.nivel;
    if nivel.is_none() {
        let grupo = *app.vista.get(i)?;
        navegar(app, Some(grupo));
        if por_tecla {
            mandar_con_teclado(app, None);
        }
        return None;
    }
    let acceso = app.acceso(nivel, i);
    cerrar(app);
    hook::terminar();
    acceso
}

/// Vuelve a la rueda principal o, si ya está en ella, la cierra.
fn volver(app: &mut App, por_tecla: bool) {
    match app.rueda.as_ref().map(|r| r.nivel) {
        Some(Some(_)) => {
            navegar(app, None);
            if por_tecla {
                mandar_con_teclado(app, None);
            }
        }
        Some(None) => {
            cerrar(app);
            hook::terminar();
        }
        None => {}
    }
}

/// Deja el mando en el teclado, con `seleccion` elegida, hasta que el puntero se mueva.
fn mandar_con_teclado(app: &mut App, seleccion: Option<usize>) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    if let Some(r) = app.rueda.as_mut() {
        r.teclado = Some((p, seleccion));
    }
    actualizar(app);
}

/// Modo clic: el izquierdo abre el grupo o la app apuntada; el derecho, o un clic en el centro,
/// vuelve a la rueda principal o, si ya está en ella, la cierra. Devuelve el acceso a abrir.
fn clic(app: &mut App, derecho: bool) -> Option<config::Acceso> {
    actualizar(app);
    match app.rueda.as_ref()?.seleccion.filter(|_| !derecho) {
        Some(i) => activar(app, i, false),
        None => {
            volver(app, false);
            None
        }
    }
}

/// Una tecla con la rueda abierta: un número abre esa baldosa, las flechas mueven la selección,
/// Intro o Espacio abren la elegida y Retroceso vuelve atrás. Devuelve el acceso a abrir.
fn tecla(app: &mut App, vk: u32) -> Option<config::Acceso> {
    actualizar(app);
    let (nivel, actual) = app.rueda.as_ref().map(|r| (r.nivel, r.seleccion))?;
    let n = app.baldosas(nivel).len();
    let es = |t: VIRTUAL_KEY| t.0 as u32 == vk;
    match vk {
        // Fila de números y teclado numérico: el 1 es la baldosa de arriba.
        0x31..=0x39 | 0x61..=0x69 => {
            let i = (vk & 0x0F) as usize - 1;
            if i < n {
                return activar(app, i, true);
            }
        }
        _ if es(VK_RETURN) || es(VK_SPACE) => {
            if let Some(i) = actual {
                return activar(app, i, true);
            }
        }
        _ if es(VK_BACK) => volver(app, true),
        _ if n > 0 => {
            let adelante = es(VK_RIGHT) || es(VK_DOWN);
            let i = match actual {
                Some(i) if adelante => (i + 1) % n,
                Some(i) => (i + n - 1) % n,
                None if adelante => 0,
                None => n - 1,
            };
            mandar_con_teclado(app, Some(i));
        }
        _ => {}
    }
    None
}

/// Recalcula la selección y el avance de la animación; solo repinta si algo cambió.
fn actualizar(app: &mut App) {
    let Some(nivel) = app.rueda.as_ref().map(|r| r.nivel) else {
        return;
    };
    let n = app.baldosas(nivel).len();
    let zona = app.cfg.zona_central;
    let espera_grupo = app.cfg.espera_grupo_ms as u128;
    let espera_volver = app.cfg.espera_volver_ms as u128;
    // En modo clic (o con la rueda abierta desde el teclado) se navega con clics, no esperando.
    let por_espera = app.cfg.modo == Modo::Soltar && !hook::por_teclado();
    let Some(r) = app.rueda.as_mut() else { return };
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    let dx = (p.x - r.origen.x) as f32 - r.centro.0;
    let dy = (p.y - r.origen.y) as f32 - r.centro.1;
    let radio_central = zona * r.escala;
    let distancia = (dx * dx + dy * dy).sqrt();
    // Con el puntero donde estaba en la última tecla, manda el teclado; si se movió, el ratón.
    let con_teclado = match r.teclado {
        Some((donde, elegida))
            if (p.x - donde.x).abs() <= HOLGURA_TECLADO && (p.y - donde.y).abs() <= HOLGURA_TECLADO =>
        {
            Some(elegida.filter(|&i| i < n))
        }
        _ => {
            r.teclado = None;
            None
        }
    };
    let seleccion = match con_teclado {
        Some(elegida) => elegida,
        None => wheel::sector_en(dx, dy, n, radio_central),
    };
    let ahora = Instant::now();
    let mut progreso_grupo = 0.0;

    // Navegación entre niveles.
    let mut cambio: Option<Option<usize>> = None;
    match r.nivel {
        _ if !por_espera || con_teclado.is_some() => {
            r.apuntado = None;
            r.en_centro = None;
        }
        // En la rueda principal todas las baldosas son grupos.
        None => match seleccion {
            Some(i) => {
                let desde = match r.apuntado {
                    Some((g, t)) if g == i => t,
                    _ => {
                        r.apuntado = Some((i, ahora));
                        ahora
                    }
                };
                let pasado = ahora.duration_since(desde).as_millis();
                progreso_grupo = (pasado as f32 / espera_grupo as f32).min(1.0);
                // Exige una dirección estable. Alejar el puntero no confirma por sí solo: ese
                // atajo abría grupos accidentalmente durante movimientos rápidos.
                if pasado >= espera_grupo {
                    cambio = app.vista.get(i).map(|&g| Some(g));
                }
            }
            None => r.apuntado = None,
        },
        Some(_) => {
            // Volver: quedarse un momento en el círculo central (soltar ahí sigue cancelando).
            if distancia < radio_central {
                let desde = *r.en_centro.get_or_insert(ahora);
                if ahora.duration_since(desde).as_millis() >= espera_volver {
                    cambio = Some(None);
                }
            } else {
                r.en_centro = None;
            }
        }
    }
    if let Some(nuevo) = cambio {
        navegar(app, nuevo);
        return;
    }

    let t = (r.inicio.elapsed().as_secs_f32() * 1000.0 / APERTURA_MS).min(1.0);
    let progreso = 1.0 - (1.0 - t).powi(3); // ease-out cúbico

    // El pulso solo hace falta si algo avanza con el tiempo aunque el ratón esté quieto: la
    // apertura o una espera (entrar en un grupo, volver desde el centro). Si no, se para y los
    // fotogramas los pide el gancho al mover el ratón.
    let con_tiempo = progreso < 1.0 || r.apuntado.is_some() || r.en_centro.is_some();
    let hwnd = app.hwnd;

    if seleccion != r.seleccion || progreso != r.progreso || progreso_grupo != r.progreso_grupo {
        r.seleccion = seleccion;
        r.progreso = progreso;
        r.progreso_grupo = progreso_grupo;
        pintar(app);
    }
    if con_tiempo {
        ritmo::empezar(hwnd);
    } else {
        ritmo::parar();
    }
}

fn pintar(app: &mut App) {
    let Some(r) = app.rueda.as_ref() else { return };
    if let Err(e) = app.renderer.preparar(r.ancho, r.alto) {
        eprintln!("preparar: {e}");
        return;
    }
    let grupo = r.nivel.and_then(|g| app.cfg.grupos.get(g)).map(|g| g.nombre.as_str());
    // Por campos y no con `app.baldosas()`: el renderer se presta mutable a la vez.
    let baldosas: &[Baldosa] = match r.nivel.and_then(|g| app.grupos.get(g)) {
        Some(b) => b,
        None => &app.raiz_vista,
    };
    let escena = Escena {
        centro: r.centro,
        escala: r.escala,
        baldosas,
        grupo,
        seleccion: r.seleccion,
        progreso: r.progreso,
        progreso_grupo: r.progreso_grupo,
        radio_central: app.cfg.zona_central * r.escala,
        mostrar_nombres: app.cfg.mostrar_nombres,
        color_hover: app.cfg.color_hover,
        opacidad_fondo: app.cfg.opacidad_fondo,
    };
    if let Err(e) = app.renderer.dibujar(&escena) {
        eprintln!("dibujar: {e}");
        return;
    }
    let tam = SIZE {
        cx: r.ancho,
        cy: r.alto,
    };
    let origen_src = POINT::default();
    let mezcla = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: (255.0 * r.progreso.max(0.15)) as u8,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    unsafe {
        if let Err(e) = UpdateLayeredWindow(
            app.hwnd,
            None,
            Some(&r.origen),
            Some(&tam),
            Some(app.renderer.dc()),
            Some(&origen_src),
            COLORREF(0),
            Some(&mezcla),
            ULW_ALPHA,
        ) {
            eprintln!("UpdateLayeredWindow: {e}");
        }
    }
}
