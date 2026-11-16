//! Ventana de ajustes, dibujada entera con Direct2D sobre un fondo acrílico translúcido.
//!
//! A la izquierda, una barra con las secciones; a la derecha, filas con título, descripción y
//! un control (interruptor, selector segmentado o deslizador). En "Accesos", todas las apps
//! instaladas con su ícono y buscador, los grupos, y la lista elegida (la rueda principal o un
//! grupo) con su editor.
//!
//! **Cómo está hecha.** Es una interfaz de modo inmediato: cada `WM_PAINT` dibuja todo desde el
//! estado y, de paso, apunta en `zonas` qué rectángulo hace qué. El ratón se resuelve contra esas
//! zonas. No hay controles nativos: los de Windows no se pueden pintar sobre el fondo acrílico
//! (GDI escribe alfa 0 y el texto sale transparente), así que los campos de texto también son
//! propios, con cursor, borrar, flechas, Inicio/Fin, Ctrl+A y Ctrl+V.
//!
//! Las coordenadas son DIP (píxeles lógicos): el render target lleva el DPI de la ventana y el
//! ratón se divide por la escala al entrar.
//!
//! Sobre el estado: vive en un `RefCell`. Los diálogos modales (Examinar) y abrir archivos se
//! hacen con el estado devuelto, porque tienen su propio bucle de mensajes.

use crate::apps::{self, AppInstalada};
use crate::autostart;
use crate::config::{self, AbrirEn, Acceso, Boton, Config, Grupo, Modo};
use crate::icons::{self, Icono, IconoRef};
use crate::launch;
use crate::lucide;
use crate::render::{self, Escena};
use crate::ui::{dentro, glifo, rect, rgba, tema, Alin, Color, Peso, Pintor, Rect};
use crate::{actualizacion, atajo, tray, ventanas};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;
use windows::core::{w, Interface, BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Controls::{MARGINS, WM_MOUSELEAVE};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Avisa a la ventana principal de que la configuración cambió (para precargar íconos).
pub const WM_ORBYT_CONFIG: u32 = WM_APP + 20;
const WM_APPS_LISTA: u32 = WM_APP + 30;
const WM_APPS_ICONOS: u32 = WM_APP + 31;
/// Llegó la respuesta de la comprobación de versiones; `lParam` es un `Box<actualizacion::Estado>`.
const WM_ACTUALIZACION: u32 = WM_APP + 32;
const TIMER_CURSOR: usize = 1;

const ANCHO_LATERAL: f32 = 212.0;
const FILA_APP: f32 = 40.0;
const FILA_ACCESO: f32 = 44.0;
/// Más accesos que estos y la rueda empieza a quedar apretada.
const RECOMENDADO: usize = 10;
/// Colores de resaltado que se ofrecen; "Otro…" abre el selector de Windows.
const COLORES: [[u8; 3]; 6] = [
    [72, 64, 96],
    [52, 92, 150],
    [40, 112, 108],
    [58, 112, 72],
    [150, 104, 44],
    [150, 60, 72],
];

static VENTANA: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Seccion {
    General,
    Activacion,
    Apariencia,
    Programas,
    Accesos,
    Acerca,
}

const SECCIONES: [(Seccion, char, &str); 6] = [
    (Seccion::General, glifo::AJUSTES, "General"),
    (Seccion::Activacion, glifo::RATON, "Activación"),
    (Seccion::Apariencia, glifo::COLOR, "Apariencia"),
    (Seccion::Programas, glifo::PROGRAMAS, "Programas"),
    (Seccion::Accesos, glifo::APPS, "Accesos"),
    (Seccion::Acerca, glifo::INFO, "Acerca de"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CampoId {
    Buscar = 0,
    Nombre = 1,
    Destino = 2,
    Args = 3,
    BuscarIcono = 4,
    Archivo = 5,
    NombreGrupo = 6,
    Excluir = 7,
    SoloEn = 8,
}

/// Pestañas del selector de íconos.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ModoSel {
    Lucide,
    Archivo,
}

/// Archivo que se abre por defecto en la pestaña "Archivo": tiene más de 300 íconos.
const ARCHIVO_ICONOS: &str = "%SystemRoot%\\System32\\shell32.dll";
const CELDA: f32 = 52.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Desliz {
    Retardo,
    Tamano,
    Zona,
    OpacidadFondo,
    EsperaGrupo,
    EsperaVolver,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Accion {
    Seccion(Seccion),
    AbrirEn(AbrirEn),
    Boton(Boton),
    Modo(Modo),
    Deslizador(Desliz),
    InicioWindows,
    Oscurecer,
    MostrarNombres,
    PausarPantallaCompleta,
    Campo(CampoId),
    AgregarApp(usize),
    Elegir(usize),
    Subir(usize),
    Bajar(usize),
    Quitar(usize),
    Examinar,
    Guardar,
    Nuevo,
    Recargar,
    AbrirConfig,
    AbrirCarpeta,
    Salir,
    AbrirSelector,
    CerrarSelector,
    /// Zona que solo tapa lo de debajo (el panel del selector).
    Bloqueo,
    IconoAuto,
    ModoSelector(ModoSel),
    ElegirLucide(usize),
    ElegirArchivo(i32),
    ExaminarIconos,
    /// Qué lista se edita: la rueda principal (`None`) o un grupo.
    Contexto(Option<usize>),
    NuevoGrupo,
    GrupoIzquierda,
    GrupoDerecha,
    QuitarGrupo,
    IconoGrupo,
    EnfocarAbiertas,
    /// Uno de los colores de `COLORES`.
    Color(usize),
    ColorOtro,
    CapturarAtajo,
    QuitarAtajo,
    AgregarExcluida,
    QuitarExcluida(usize),
    /// Excluir un programa de la lista de abiertos (posición en `abiertos`).
    ExcluirAbierto(usize),
    RecargarAbiertos,
    /// Casilla "Como administrador" del editor.
    Admin,
    BuscarActualizacion,
    AbrirDescarga,
    Exportar,
    Importar,
}

/// Lo que hay que hacer después de soltar el estado (diálogos con su propio bucle de mensajes).
enum Diferida {
    Examinar,
    ExaminarIconos,
    Abrir(Acceso),
    Color,
    Exportar,
    Importar,
}

/// Lo que se está arrastrando para reordenar: un acceso de la lista o la pastilla de un grupo.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Que {
    Fila(usize),
    Grupo(usize),
}

#[derive(Clone, Copy)]
struct Arrastrando {
    que: Que,
    /// Dónde se pulsó: hasta que el puntero no se aleja un poco, es un clic y no un arrastre.
    origen: (f32, f32),
    /// Posición en la que quedaría al soltar.
    destino: usize,
    activo: bool,
}
mod accesos;
mod acciones;
mod campo;
mod controles;
mod programas;
mod secciones;
mod selector;
mod teclado;

use campo::Campo;

// ─── Estado ─────────────────────────────────────────────────────────────────────────────────

struct Zona {
    r: Rect,
    accion: Accion,
}

struct Ajustes {
    hwnd: HWND,
    principal: HWND,
    factory: ID2D1Factory,
    dwrite: IDWriteFactory,
    rt: Option<ID2D1HwndRenderTarget>,
    pintor: Option<Pintor>,
    /// El fondo acrílico está activo (Windows 11). Si no, se pinta un fondo opaco.
    vidrio: bool,
    escala: f32,

    seccion: Seccion,
    cfg: Config,
    inicio_windows: bool,

    apps: Vec<AppInstalada>,
    cargando: bool,
    /// Posiciones en `apps` que pasan el filtro de búsqueda, en orden.
    filtradas: Vec<usize>,

    campos: [Campo; 9],
    foco: Option<CampoId>,
    cursor_visible: bool,
    /// Primera mitad de un carácter fuera del plano básico (WM_CHAR llega en dos partes).
    sustituto: Option<u16>,
    seleccion: Option<usize>,
    /// Ícono elegido en el editor (`lucide:...` o `archivo:...`); `None` es el automático.
    icono_editor: Option<String>,
    /// Casilla "Como administrador" del editor.
    admin_editor: bool,
    /// Se está esperando la combinación de teclas del atajo.
    capturando: bool,
    /// Programas con ventanas abiertas, para ofrecerlos en las exclusiones.
    abiertos: Vec<String>,
    actualizacion: actualizacion::Estado,
    arrastrando: Option<Arrastrando>,
    /// Pastillas de los grupos tal como se pintaron: (grupo, rectángulo).
    pastillas: Vec<(usize, Rect)>,
    /// Grupo cuya lista se edita; `None` es la rueda principal.
    grupo: Option<usize>,
    /// El selector de íconos está eligiendo el del grupo, no el del acceso.
    selector_grupo: bool,
    /// Se pulsó "quitar grupo" una vez: el segundo clic lo borra.
    confirmar_quitar: bool,

    selector: Option<ModoSel>,
    scroll_sel: f32,
    max_sel: f32,
    area_sel: Rect,
    lucide_filtrados: Vec<usize>,
    archivo_cantidad: u32,
    iconos_archivo: HashMap<(String, i32), Option<IconoRef>>,

    zonas: Vec<Zona>,
    raton: (f32, f32),
    hover: Option<Accion>,
    pulsada: Option<Accion>,
    arrastre: Option<Desliz>,
    pistas: Vec<(Desliz, Rect)>,
    campos_r: Vec<(CampoId, Rect, f32)>,

    scroll_apps: f32,
    scroll_accesos: f32,
    area_apps: Rect,
    area_accesos: Rect,
    max_apps: f32,
    max_accesos: f32,
}

thread_local! {
    static AJUSTES: RefCell<Option<Ajustes>> = const { RefCell::new(None) };
}

fn con_ajustes<R>(f: impl FnOnce(&mut Ajustes) -> R) -> Option<R> {
    AJUSTES.with(|a| match a.try_borrow_mut() {
        Ok(mut g) => g.as_mut().map(f),
        Err(_) => None,
    })
}

/// Abre la ventana, o la trae al frente si ya estaba abierta.
pub fn abrir(principal: HWND) {
    let v = VENTANA.load(Ordering::Relaxed);
    if v != 0 {
        unsafe {
            let h = HWND(v as *mut _);
            let _ = ShowWindow(h, SW_RESTORE);
            let _ = SetForegroundWindow(h);
        }
        return;
    }
    if let Err(e) = crear(principal) {
        eprintln!("no se pudo abrir los ajustes: {e}");
    }
}

/// Cierra la ventana si está abierta (al salir de Orbyt).
pub fn cerrar() {
    let v = VENTANA.load(Ordering::Relaxed);
    if v != 0 {
        unsafe {
            let _ = DestroyWindow(HWND(v as *mut _));
        }
    }
}

/// Repinta la ventana si está abierta (por ejemplo, porque llegó un favicon).
pub fn refrescar() {
    con_ajustes(|a| a.invalidar());
}

fn crear(principal: HWND) -> windows::core::Result<()> {
    unsafe {
        let instancia: HINSTANCE = GetModuleHandleW(None)?.into();
        let clase = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instancia,
            lpszClassName: w!("OrbytAjustes"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hIcon: LoadIconW(None, IDI_APPLICATION)?,
            ..Default::default()
        };
        RegisterClassW(&clase);

        // Tamaño inicial según el DPI del monitor donde está el puntero, centrado en él.
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let monitor = MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let s = dx as f32 / 96.0;
        let t = info.rcWork;
        let (an, al) = (
            ((1000.0 * s) as i32).min(t.right - t.left),
            ((800.0 * s) as i32).min(t.bottom - t.top),
        );
        let (x, y) = (
            t.left + (t.right - t.left - an) / 2,
            t.top + (t.bottom - t.top - al) / 2,
        );

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("OrbytAjustes"),
            w!("Orbyt"),
            WS_OVERLAPPEDWINDOW,
            x,
            y,
            an,
            al,
            None,
            None,
            Some(instancia),
            None,
        )?;

        // Barra de título oscura y fondo acrílico. En Windows 10 el fondo falla y se usa uno opaco.
        let oscuro = BOOL(1);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &oscuro as *const _ as *const _, 4);
        let fondo = DWMSBT_TRANSIENTWINDOW;
        let vidrio = DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &fondo as *const _ as *const _, 4).is_ok()
            && DwmExtendFrameIntoClientArea(
                hwnd,
                &MARGINS {
                    cxLeftWidth: -1,
                    cxRightWidth: -1,
                    cyTopHeight: -1,
                    cyBottomHeight: -1,
                },
            )
            .is_ok();

        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let mut a = Ajustes {
            hwnd,
            principal,
            factory,
            dwrite,
            rt: None,
            pintor: None,
            vidrio,
            escala: GetDpiForWindow(hwnd).max(96) as f32 / 96.0,
            seccion: Seccion::Accesos,
            cfg: config::cargar(),
            inicio_windows: autostart::activo(),
            apps: Vec::new(),
            cargando: true,
            filtradas: Vec::new(),
            campos: Default::default(),
            foco: None,
            cursor_visible: true,
            sustituto: None,
            seleccion: None,
            icono_editor: None,
            admin_editor: false,
            capturando: false,
            abiertos: Vec::new(),
            actualizacion: actualizacion::Estado::SinComprobar,
            arrastrando: None,
            pastillas: Vec::new(),
            grupo: None,
            selector_grupo: false,
            confirmar_quitar: false,
            selector: None,
            scroll_sel: 0.0,
            max_sel: 0.0,
            area_sel: rect(0.0, 0.0, 0.0, 0.0),
            lucide_filtrados: lucide::buscar(""),
            archivo_cantidad: 0,
            iconos_archivo: HashMap::new(),
            zonas: Vec::new(),
            raton: (-1.0, -1.0),
            hover: None,
            pulsada: None,
            arrastre: None,
            pistas: Vec::new(),
            campos_r: Vec::new(),
            scroll_apps: 0.0,
            scroll_accesos: 0.0,
            area_apps: rect(0.0, 0.0, 0.0, 0.0),
            area_accesos: rect(0.0, 0.0, 0.0, 0.0),
            max_apps: 0.0,
            max_accesos: 0.0,
        };
        a.campo(CampoId::Archivo).poner(ARCHIVO_ICONOS);
        match apps::en_cache() {
            Some(lista) => {
                a.apps = lista;
                a.cargando = false;
            }
            None => apps::cargar_en_segundo_plano(hwnd, WM_APPS_LISTA, WM_APPS_ICONOS),
        }
        a.filtrar();
        a.ir_a((!a.cfg.grupos.is_empty()).then_some(0));

        AJUSTES.with(|s| *s.borrow_mut() = Some(a));
        VENTANA.store(hwnd.0 as isize, Ordering::Relaxed);
        SetTimer(Some(hwnd), TIMER_CURSOR, 530, None);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

// ─── Dibujo ─────────────────────────────────────────────────────────────────────────────────

impl Ajustes {
    fn asegurar_rt(&mut self) -> Option<ID2D1HwndRenderTarget> {
        if let Some(rt) = &self.rt {
            return Some(rt.clone());
        }
        let mut cr = RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut cr);
        }
        let dpi = self.escala * 96.0;
        let props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: dpi,
            dpiY: dpi,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let hprops = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: self.hwnd,
            pixelSize: D2D_SIZE_U {
                width: cr.right.max(1) as u32,
                height: cr.bottom.max(1) as u32,
            },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        let rt = unsafe { self.factory.CreateHwndRenderTarget(&props, &hprops).ok()? };
        self.pintor = Some(Pintor::nuevo(rt.cast().ok()?, self.dwrite.clone()).ok()?);
        self.rt = Some(rt.clone());
        Some(rt)
    }

    fn pintar(&mut self) {
        let Some(rt) = self.asegurar_rt() else { return };
        let Some(mut p) = self.pintor.take() else { return };
        let mut cr = RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut cr);
        }
        let (w, h) = (cr.right as f32 / self.escala, cr.bottom as f32 / self.escala);
        self.zonas.clear();
        self.pistas.clear();
        self.campos_r.clear();
        self.pastillas.clear();
        unsafe {
            rt.BeginDraw();
            rt.SetTransform(&windows_numerics::Matrix3x2::identity());
            rt.Clear(Some(&tema::TRANSPARENTE));
        }
        // Sobre el acrílico, un velo oscuro que le da el tono; sin acrílico, un fondo opaco.
        let (fondo, lateral) = if self.vidrio {
            (rgba(14, 14, 16, 0.58), rgba(255, 255, 255, 0.025))
        } else {
            (rgba(24, 24, 27, 1.0), rgba(30, 30, 34, 1.0))
        };
        p.rellenar(rect(0.0, 0.0, w, h), 0.0, fondo);
        p.rellenar(rect(0.0, 0.0, ANCHO_LATERAL, h), 0.0, lateral);
        p.linea(ANCHO_LATERAL, 0.0, ANCHO_LATERAL, h, tema::SEPARADOR, 1.0);

        self.lateral(&mut p, h);
        let x = ANCHO_LATERAL + 32.0;
        let cw = (w - x - 32.0).max(200.0);
        match self.seccion {
            Seccion::General => self.general(&mut p, x, cw),
            Seccion::Activacion => self.activacion(&mut p, x, cw),
            Seccion::Apariencia => self.apariencia(&mut p, x, cw, h),
            Seccion::Programas => self.programas(&mut p, x, cw),
            Seccion::Accesos => self.accesos(&mut p, x, cw, h),
            Seccion::Acerca => self.acerca(&mut p, x, cw),
        }
        if self.seccion == Seccion::Accesos && self.selector.is_some() {
            self.selector_ui(&mut p, w, h);
        }

        let r = unsafe { rt.EndDraw(None, None) };
        self.pintor = Some(p);
        if r.is_err() {
            self.rt = None;
            self.pintor = None;
        }
    }
}

fn examinar(dueño: HWND, filtro: PCWSTR, titulo: PCWSTR) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: dueño,
        lpstrFilter: filtro,
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrTitle: titulo,
        // Sin seguir los .lnk: el acceso directo conserva sus argumentos y su ícono.
        Flags: OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR | OFN_NODEREFERENCELINKS | OFN_EXPLORER,
        ..Default::default()
    };
    unsafe {
        if !GetOpenFileNameW(&mut ofn).as_bool() {
            return None;
        }
    }
    let fin = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..fin]))
}

/// Diálogo "Guardar como" para exportar la configuración.
fn guardar_como(dueño: HWND) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    for (hueco, unidad) in buf.iter_mut().zip("orbyt-config.json".encode_utf16()) {
        *hueco = unidad;
    }
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: dueño,
        lpstrFilter: w!("Configuración de Orbyt (*.json)\0*.json\0\0"),
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrTitle: w!("Exportar la configuración"),
        lpstrDefExt: w!("json"),
        Flags: OFN_OVERWRITEPROMPT | OFN_NOCHANGEDIR | OFN_EXPLORER,
        ..Default::default()
    };
    unsafe {
        if !GetSaveFileNameW(&mut ofn).as_bool() {
            return None;
        }
    }
    let fin = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..fin]))
}

/// Selector de color de Windows, abierto en el color actual.
fn elegir_color(dueño: HWND, actual: [u8; 3]) -> Option<[u8; 3]> {
    let mut propios = [COLORREF(0x00FF_FFFF); 16];
    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: dueño,
        rgbResult: COLORREF(actual[0] as u32 | (actual[1] as u32) << 8 | (actual[2] as u32) << 16),
        lpCustColors: propios.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        ..Default::default()
    };
    unsafe {
        if !ChooseColorW(&mut cc).as_bool() {
            return None;
        }
    }
    let c = cc.rgbResult.0;
    let color = [c as u8, (c >> 8) as u8, (c >> 16) as u8];
    // El negro puro significa "el color por defecto" al dibujar: se guarda casi negro.
    Some(if color == [0, 0, 0] { [1, 1, 1] } else { color })
}

fn hacer(d: Diferida, hwnd: HWND) {
    match d {
        Diferida::Color => {
            let actual = con_ajustes(|a| a.cfg.color_hover).unwrap_or_default();
            if let Some(color) = elegir_color(hwnd, actual) {
                con_ajustes(|a| {
                    a.cfg.color_hover = color;
                    a.guardar();
                    a.invalidar();
                });
            }
        }
        Diferida::Exportar => {
            if let Some(ruta) = guardar_como(hwnd) {
                let resultado = con_ajustes(|a| config::exportar(&a.cfg, std::path::Path::new(&ruta)));
                if let Some(Err(e)) = resultado {
                    tray::avisar("No se pudo exportar la configuración", &e.to_string());
                }
            }
        }
        Diferida::Importar => {
            let filtro = w!("Configuración de Orbyt (*.json)\0*.json\0Todos los archivos\0*.*\0\0");
            if let Some(ruta) = examinar(hwnd, filtro, w!("Importar una configuración")) {
                match config::importar(std::path::Path::new(&ruta)) {
                    Ok(cfg) => {
                        // La configuración que se va a reemplazar queda guardada al lado.
                        let actual = config::ruta();
                        let _ = std::fs::copy(&actual, actual.with_file_name("config.anterior.json"));
                        con_ajustes(|a| {
                            a.cfg = cfg;
                            a.guardar();
                            a.ir_a((!a.cfg.grupos.is_empty()).then_some(0));
                            a.invalidar();
                        });
                    }
                    Err(e) => tray::avisar("No se pudo importar la configuración", &e),
                }
            }
        }
        Diferida::ExaminarIconos => {
            let filtro = w!("Archivos con íconos\0*.dll;*.exe;*.ico;*.cpl;*.mun\0Todos los archivos\0*.*\0\0");
            if let Some(ruta) = examinar(hwnd, filtro, w!("Elegir un archivo con íconos")) {
                con_ajustes(|a| {
                    a.campo(CampoId::Archivo).poner(&ruta);
                    a.cargar_archivo();
                    a.invalidar();
                });
            }
        }
        Diferida::Examinar => {
            let filtro = w!("Programas y accesos directos\0*.exe;*.lnk;*.bat;*.cmd;*.ps1\0Todos los archivos\0*.*\0\0");
            if let Some(ruta) = examinar(hwnd, filtro, w!("Elegir un archivo para la rueda")) {
                con_ajustes(|a| {
                    if a.campos[CampoId::Nombre as usize].texto.trim().is_empty() {
                        let nombre = std::path::Path::new(&ruta)
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        a.campo(CampoId::Nombre).poner(&nombre);
                    }
                    a.campo(CampoId::Destino).poner(&ruta);
                    a.invalidar();
                });
            }
        }
        Diferida::Abrir(acceso) => launch::abrir(&acceso, false),
    }
}

// ─── Procedimiento de ventana ───────────────────────────────────────────────────────────────

fn punto(lparam: LPARAM) -> (i32, i32) {
    (
        (lparam.0 & 0xFFFF) as i16 as i32,
        ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
    )
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => return LRESULT(1),
        WM_GETMINMAXINFO => unsafe {
            let s = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
            let info = &mut *(lparam.0 as *mut MINMAXINFO);
            info.ptMinTrackSize = POINT {
                x: (860.0 * s) as i32,
                y: (660.0 * s) as i32,
            };
            return LRESULT(0);
        },
        WM_APPS_LISTA => {
            let lista = unsafe { Box::from_raw(lparam.0 as *mut Vec<AppInstalada>) };
            con_ajustes(|a| {
                a.apps = *lista;
                a.cargando = false;
                a.filtrar();
                a.invalidar();
            });
            return LRESULT(0);
        }
        WM_APPS_ICONOS => {
            let tanda = unsafe { Box::from_raw(lparam.0 as *mut Vec<(usize, Option<icons::IconoRef>)>) };
            con_ajustes(|a| {
                for (i, ic) in *tanda {
                    if let Some(app) = a.apps.get_mut(i) {
                        app.icono = ic;
                    }
                }
                a.invalidar();
            });
            return LRESULT(0);
        }
        WM_ACTUALIZACION => {
            let estado = unsafe { Box::from_raw(lparam.0 as *mut actualizacion::Estado) };
            con_ajustes(|a| {
                a.actualizacion = *estado;
                a.invalidar();
            });
            return LRESULT(0);
        }
        WM_DESTROY => {
            VENTANA.store(0, Ordering::Relaxed);
            AJUSTES.with(|s| s.borrow_mut().take());
            return LRESULT(0);
        }
        _ => {}
    }

    let mut diferida = None;
    let manejado = con_ajustes(|a| unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                BeginPaint(hwnd, &mut ps);
                a.pintar();
                let _ = EndPaint(hwnd, &ps);
            }
            WM_SIZE => {
                if let Some(rt) = &a.rt {
                    let (w, h) = ((lparam.0 & 0xFFFF) as u32, ((lparam.0 >> 16) & 0xFFFF) as u32);
                    let _ = rt.Resize(&D2D_SIZE_U {
                        width: w.max(1),
                        height: h.max(1),
                    });
                }
                a.invalidar();
            }
            WM_DPICHANGED => {
                a.escala = ((wparam.0 & 0xFFFF) as f32 / 96.0).max(1.0);
                a.rt = None;
                a.pintor = None;
                let r = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                a.invalidar();
            }
            WM_TIMER if wparam.0 == TIMER_CURSOR => {
                if a.foco.is_some() {
                    // El parpadeo repinta la ventana entera: solo vale la pena si se ve. En
                    // segundo plano o minimizada, el cursor queda fijo y no se repinta más.
                    let visible = GetForegroundWindow() == hwnd && !IsIconic(hwnd).as_bool();
                    if visible || !a.cursor_visible {
                        a.cursor_visible = !a.cursor_visible || !visible;
                        a.invalidar();
                    }
                }
            }
            WM_MOUSEMOVE => {
                let (x, y) = punto(lparam);
                a.raton = (x as f32 / a.escala, y as f32 / a.escala);
                if let Some(d) = a.arrastre {
                    a.poner_deslizador(d, a.raton.0);
                    a.invalidar();
                }
                if let Some(mut ar) = a.arrastrando {
                    let lejos = (a.raton.0 - ar.origen.0).abs() > 5.0 || (a.raton.1 - ar.origen.1).abs() > 5.0;
                    ar.activo |= lejos;
                    if ar.activo {
                        ar.destino = a.destino_arrastre(ar.que).unwrap_or(ar.destino);
                        a.invalidar();
                    }
                    a.arrastrando = Some(ar);
                }
                let nuevo = a.zona_en(a.raton.0, a.raton.1);
                if nuevo != a.hover {
                    a.hover = nuevo;
                    a.invalidar();
                }
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut tme);
            }
            WM_MOUSELEAVE => {
                a.hover = None;
                a.invalidar();
            }
            WM_SETCURSOR if (lparam.0 & 0xFFFF) as u32 == HTCLIENT => {
                let cursor = match a.hover {
                    Some(Accion::Campo(_)) => IDC_IBEAM,
                    Some(_) => IDC_HAND,
                    None => IDC_ARROW,
                };
                if let Ok(c) = LoadCursorW(None, cursor) {
                    SetCursor(Some(c));
                }
            }
            WM_LBUTTONDOWN => {
                let (x, y) = punto(lparam);
                a.raton = (x as f32 / a.escala, y as f32 / a.escala);
                SetCapture(hwnd);
                let accion = a.zona_en(a.raton.0, a.raton.1);
                a.pulsada = accion;
                match accion {
                    Some(Accion::Deslizador(d)) => {
                        a.arrastre = Some(d);
                        a.poner_deslizador(d, a.raton.0);
                    }
                    Some(Accion::Campo(id)) => {
                        a.foco = Some(id);
                        a.cursor_visible = true;
                        a.cursor_en(id, a.raton.0);
                    }
                    // Un acceso o un grupo se pueden arrastrar para cambiarlos de sitio.
                    Some(Accion::Elegir(i)) => {
                        a.foco = None;
                        a.arrastrando = Some(Arrastrando {
                            que: Que::Fila(i),
                            origen: a.raton,
                            destino: i,
                            activo: false,
                        });
                    }
                    Some(Accion::Contexto(Some(g))) => {
                        a.foco = None;
                        a.arrastrando = Some(Arrastrando {
                            que: Que::Grupo(g),
                            origen: a.raton,
                            destino: g,
                            activo: false,
                        });
                    }
                    _ => a.foco = None,
                }
                a.invalidar();
            }
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                let soltado = a.arrastrando.take().filter(|ar| ar.activo);
                if a.arrastre.take().is_some() {
                    a.guardar();
                } else if let Some(ar) = soltado {
                    a.soltar_arrastre(ar);
                } else {
                    let accion = a.zona_en(a.raton.0, a.raton.1);
                    if let Some(acc) = accion.filter(|x| Some(*x) == a.pulsada) {
                        if !matches!(acc, Accion::Campo(_)) {
                            diferida = a.ejecutar(acc);
                        }
                    }
                }
                a.pulsada = None;
                a.hover = a.zona_en(a.raton.0, a.raton.1);
                a.invalidar();
            }
            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32 / 120.0;
                let mut pt = POINT {
                    x: punto(lparam).0,
                    y: punto(lparam).1,
                };
                let _ = ScreenToClient(hwnd, &mut pt);
                let (x, y) = (pt.x as f32 / a.escala, pt.y as f32 / a.escala);
                if a.selector.is_some() {
                    if dentro(&a.area_sel, x, y) {
                        a.scroll_sel = (a.scroll_sel - delta * CELDA * 2.0).clamp(0.0, a.max_sel);
                    }
                } else if dentro(&a.area_apps, x, y) && a.seccion == Seccion::Accesos {
                    a.scroll_apps = (a.scroll_apps - delta * FILA_APP * 3.0).clamp(0.0, a.max_apps);
                } else if dentro(&a.area_accesos, x, y) && a.seccion == Seccion::Accesos {
                    a.scroll_accesos = (a.scroll_accesos - delta * FILA_ACCESO * 2.0).clamp(0.0, a.max_accesos);
                }
                a.hover = a.zona_en(a.raton.0, a.raton.1);
                a.invalidar();
            }
            WM_CHAR => {
                diferida = a.caracter(wparam.0 as u16);
                a.invalidar();
            }
            WM_KEYDOWN | WM_SYSKEYDOWN if a.capturando => {
                a.capturar(wparam.0 as u32);
                // La tecla ya dejó su carácter en la cola (Escape cerraría la ventana, Alt+letra
                // abriría el menú del sistema): se retira.
                let mut m = MSG::default();
                let _ = PeekMessageW(&mut m, Some(hwnd), WM_CHAR, WM_DEADCHAR, PM_REMOVE);
                let _ = PeekMessageW(&mut m, Some(hwnd), WM_SYSCHAR, WM_SYSDEADCHAR, PM_REMOVE);
                a.invalidar();
            }
            WM_KEYDOWN => {
                if !a.tecla(wparam.0 as u16) {
                    return false;
                }
                a.invalidar();
            }
            _ => return false,
        }
        true
    });
    if let Some(d) = diferida.take() {
        hacer(d, hwnd);
    }
    if manejado == Some(true) {
        // WM_SETCURSOR tiene que decir que ya puso el cursor.
        return LRESULT(if msg == WM_SETCURSOR { 1 } else { 0 });
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
