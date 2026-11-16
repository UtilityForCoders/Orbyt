//! Configuración en `%APPDATA%\Orbyt\config.json`.
//!
//! Si el archivo no existe se crea con unos accesos de ejemplo, para que la rueda tenga algo que
//! mostrar la primera vez. Un JSON mal escrito no tumba la app: se avisa por consola (en debug) y
//! se usan los valores por defecto, sin sobrescribir el archivo del usuario.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AbrirEn {
    /// La rueda aparece centrada bajo el puntero.
    Cursor,
    /// La rueda aparece en el centro del monitor donde está el puntero.
    Centro,
}

/// Botón del ratón que abre la rueda al mantenerlo pulsado.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Boton {
    /// La rueda del ratón (botón central).
    Central,
    /// Botón lateral "atrás" (XBUTTON1).
    Atras,
    /// Botón lateral "adelante" (XBUTTON2).
    Adelante,
}

/// Cómo se elige una app o un grupo con la rueda abierta.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Modo {
    /// Se mantiene el botón, se apunta y se suelta.
    Soltar,
    /// El botón solo abre la rueda, que queda abierta al soltarlo: se elige con clic izquierdo y
    /// se vuelve (o se cierra) con el derecho.
    Clic,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Acceso {
    /// Texto que se ve en el sector.
    pub nombre: String,
    /// Programa, carpeta, archivo, URL o URI (`ms-settings:`). Acepta variables `%VAR%`.
    pub destino: String,
    /// Argumentos opcionales, por ejemplo `/k echo hola` para `cmd.exe`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argumentos: Option<String>,
    /// Ícono elegido a mano: `lucide:<nombre>` o `archivo:<ruta>,<índice>`. Sin él, el automático
    /// (el de la app, el favicon de la web...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icono: Option<String>,
    /// Abrir como administrador (Windows pide confirmación cada vez).
    #[serde(default, skip_serializing_if = "es_falso")]
    pub admin: bool,
}

fn es_falso(v: &bool) -> bool {
    !*v
}

/// Un grupo de accesos: en la rueda es una baldosa que, al elegirla, se abre y muestra sus
/// accesos. La rueda principal solo tiene grupos.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Grupo {
    pub nombre: String,
    /// Ícono elegido a mano (`lucide:...` o `archivo:...`). Sin él, la baldosa muestra en
    /// miniatura los íconos de sus primeros accesos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icono: Option<String>,
    #[serde(default)]
    pub accesos: Vec<Acceso>,
    /// Programas (por el nombre de su ejecutable, `code.exe`) en los que aparece el grupo. Vacío:
    /// aparece siempre. Así la rueda cambia según el programa que haya delante.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub solo_en: Vec<String>,
}

impl Grupo {
    /// ¿Se muestra el grupo con este programa delante? `exe` va en minúsculas.
    pub fn visible_en(&self, exe: Option<&str>) -> bool {
        self.solo_en.iter().all(|s| s.trim().is_empty())
            || exe.is_some_and(|e| self.solo_en.iter().any(|s| s.trim().eq_ignore_ascii_case(e)))
    }
}

/// Separa una lista escrita a mano (`code.exe, blender.exe`) en nombres de ejecutable en
/// minúsculas, sin repetidos ni vacíos.
pub fn lista_programas(texto: &str) -> Vec<String> {
    let mut lista: Vec<String> = Vec::new();
    for parte in texto.split([',', ';']) {
        let nombre = parte.trim().trim_matches('"').trim().to_lowercase();
        if !nombre.is_empty() && !lista.contains(&nombre) {
            lista.push(nombre);
        }
    }
    lista
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "abrir_en_defecto")]
    pub abrir_en: AbrirEn,
    /// Cuánto hay que mantener el botón central antes de que salga la rueda. Una pulsación más
    /// corta se reenvía como clic central normal (abrir enlaces en otra pestaña, etc.).
    #[serde(default = "retardo_defecto")]
    pub retardo_ms: u32,
    #[serde(default = "boton_defecto")]
    pub boton: Boton,
    #[serde(default = "modo_defecto")]
    pub modo: Modo,
    /// Atajo de teclado que abre la rueda (`Ctrl+Alt+Espacio`), además del botón del ratón. Con
    /// él la rueda queda abierta y se elige con clic o con el teclado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atajo: Option<String>,
    /// En modo soltar, cuánto hay que apuntar a un grupo para que se abra.
    #[serde(default = "espera_grupo_defecto")]
    pub espera_grupo_ms: u32,
    /// En modo soltar, cuánto hay que quedarse en el centro de un grupo para volver.
    #[serde(default = "espera_volver_defecto")]
    pub espera_volver_ms: u32,
    /// Si el programa de un acceso ya tiene una ventana abierta, se trae al frente en lugar de
    /// abrir otra.
    #[serde(default = "si")]
    pub enfocar_abiertas: bool,
    /// Escala de la rueda: 1.0 es el tamaño normal.
    #[serde(default = "tamano_defecto")]
    pub tamano: f32,
    /// Radio (en píxeles lógicos) del círculo central donde soltar cancela.
    #[serde(default = "zona_defecto")]
    pub zona_central: f32,
    /// Oscurecer un poco la pantalla mientras la rueda está abierta.
    #[serde(default = "si")]
    pub oscurecer: bool,
    #[serde(default = "mostrar_nombres_defecto")]
    pub mostrar_nombres: bool,
    /// Con una ventana a pantalla completa delante (un juego, un vídeo), Orbyt no actúa y el
    /// botón llega tal cual: muchos juegos usan el botón central mantenido.
    #[serde(default = "si")]
    pub pausar_pantalla_completa: bool,
    /// Programas en los que Orbyt no actúa, por el nombre de su ejecutable (`blender.exe`): los
    /// que usan el botón mantenido para desplazar o girar la vista.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluidas: Vec<String>,
    #[serde(default = "opacidad_fondo_defecto")]
    pub opacidad_fondo: f32, // 0.1–1.0, afecta fondo de segmentos
    /// Color (RGB) del botón al que se apunta.
    #[serde(default = "color_hover_defecto")]
    pub color_hover: [u8; 3],
    /// Los grupos de la rueda principal.
    #[serde(default)]
    pub grupos: Vec<Grupo>,
    /// Accesos sueltos de versiones anteriores. Al cargar pasan a un grupo "Principal" y ya no se
    /// escriben.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accesos: Vec<Acceso>,
}

fn abrir_en_defecto() -> AbrirEn {
    AbrirEn::Cursor
}

fn retardo_defecto() -> u32 {
    160
}

fn boton_defecto() -> Boton {
    Boton::Central
}

fn modo_defecto() -> Modo {
    Modo::Soltar
}

fn espera_grupo_defecto() -> u32 {
    450
}

fn espera_volver_defecto() -> u32 {
    300
}

fn tamano_defecto() -> f32 {
    1.0
}

fn zona_defecto() -> f32 {
    30.0
}

fn si() -> bool {
    true
}

fn mostrar_nombres_defecto() -> bool {
    false
}

fn opacidad_fondo_defecto() -> f32 {
    0.88
}

fn color_hover_defecto() -> [u8; 3] {
    [72, 64, 96]
}

pub const RETARDO_MIN: u32 = 80;
pub const RETARDO_MAX: u32 = 600;
pub const TAMANO_MIN: f32 = 0.8;
pub const TAMANO_MAX: f32 = 1.4;
pub const ZONA_MIN: f32 = 20.0;
pub const ZONA_MAX: f32 = 60.0;
pub const OPACIDAD_MIN: f32 = 0.1;
pub const OPACIDAD_MAX: f32 = 1.0;
pub const ESPERA_GRUPO_MIN: u32 = 150;
pub const ESPERA_GRUPO_MAX: u32 = 1500;
pub const ESPERA_VOLVER_MIN: u32 = 100;
pub const ESPERA_VOLVER_MAX: u32 = 1000;

impl Config {
    /// Lleva cada valor a su rango; un JSON editado a mano no puede romper la rueda.
    pub fn normalizar(&mut self) {
        self.retardo_ms = self.retardo_ms.clamp(RETARDO_MIN, RETARDO_MAX);
        self.espera_grupo_ms = self.espera_grupo_ms.clamp(ESPERA_GRUPO_MIN, ESPERA_GRUPO_MAX);
        self.espera_volver_ms = self.espera_volver_ms.clamp(ESPERA_VOLVER_MIN, ESPERA_VOLVER_MAX);
        self.atajo = self
            .atajo
            .take()
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty());
        self.excluidas = lista_programas(&self.excluidas.join(","));
        for g in &mut self.grupos {
            g.solo_en = lista_programas(&g.solo_en.join(","));
        }
        self.tamano = if self.tamano.is_finite() {
            self.tamano.clamp(TAMANO_MIN, TAMANO_MAX)
        } else {
            1.0
        };
        self.zona_central = if self.zona_central.is_finite() {
            self.zona_central.clamp(ZONA_MIN, ZONA_MAX)
        } else {
            zona_defecto()
        };
        self.opacidad_fondo = if self.opacidad_fondo.is_finite() {
            self.opacidad_fondo.clamp(OPACIDAD_MIN, OPACIDAD_MAX)
        } else {
            opacidad_fondo_defecto()
        };
        // Los accesos sueltos de antes se vuelven un grupo más, editable y que se puede quitar.
        if !self.accesos.is_empty() {
            let accesos = std::mem::take(&mut self.accesos);
            self.grupos.insert(
                0,
                Grupo {
                    nombre: "Principal".into(),
                    accesos,
                    ..Default::default()
                },
            );
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let acceso = |nombre: &str, destino: &str| Acceso {
            nombre: nombre.into(),
            destino: destino.into(),
            ..Default::default()
        };
        Config {
            abrir_en: abrir_en_defecto(),
            retardo_ms: retardo_defecto(),
            boton: boton_defecto(),
            modo: modo_defecto(),
            atajo: None,
            espera_grupo_ms: espera_grupo_defecto(),
            espera_volver_ms: espera_volver_defecto(),
            enfocar_abiertas: true,
            tamano: tamano_defecto(),
            zona_central: zona_defecto(),
            oscurecer: true,
            mostrar_nombres: mostrar_nombres_defecto(),
            pausar_pantalla_completa: true,
            excluidas: Vec::new(),
            opacidad_fondo: opacidad_fondo_defecto(),
            color_hover: color_hover_defecto(),
            grupos: vec![
                Grupo {
                    nombre: "Sistema".into(),
                    icono: Some("lucide:monitor".into()),
                    accesos: vec![
                        acceso("Explorador", "explorer.exe"),
                        acceso("Navegador", "https://www.google.com"),
                        acceso("Bloc de notas", "notepad.exe"),
                        acceso("Calculadora", "calc.exe"),
                        acceso("Terminal", "cmd.exe"),
                        acceso("Descargas", "%USERPROFILE%\\Downloads"),
                        acceso("Ajustes", "ms-settings:"),
                    ],
                    ..Default::default()
                },
                Grupo {
                    nombre: "Multimedia".into(),
                    icono: Some("lucide:clapperboard".into()),
                    accesos: vec![
                        acceso("YouTube", "https://www.youtube.com"),
                        acceso("Twitch", "https://www.twitch.tv"),
                        acceso("Kick", "https://kick.com"),
                    ],
                    ..Default::default()
                },
                Grupo {
                    nombre: "Agentes de IA".into(),
                    icono: Some("lucide:bot".into()),
                    accesos: vec![
                        acceso("Claude", "https://claude.ai"),
                        acceso("ChatGPT", "https://chatgpt.com"),
                        acceso("Gemini", "https://gemini.google.com"),
                    ],
                    ..Default::default()
                },
            ],
            accesos: Vec::new(),
        }
    }
}

pub fn ruta() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Orbyt").join("config.json")
}

/// Lee la configuración. Si no hay archivo, lo crea con los valores por defecto.
pub fn cargar() -> Config {
    leer().0
}

/// Como `cargar`, pero además devuelve el error si `config.json` está mal escrito (y por eso se
/// están usando los valores por defecto), para poder avisar.
pub fn leer() -> (Config, Option<String>) {
    let ruta = ruta();
    match std::fs::read_to_string(&ruta) {
        Ok(texto) => match interpretar(&texto) {
            Ok(cfg) => (cfg, None),
            Err(e) => {
                // Se guarda una copia: al cambiar algo en los ajustes se escribe un config.json
                // nuevo, y sin ella se perderían los accesos del archivo mal escrito.
                let _ = std::fs::copy(&ruta, ruta.with_file_name("config.invalido.json"));
                (Config::default(), Some(e))
            }
        },
        Err(_) => {
            let cfg = Config::default();
            if let Some(dir) = ruta.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(&cfg) {
                let _ = escribir(&ruta, &json);
            }
            (cfg, None)
        }
    }
}

/// Convierte el texto de un `config.json` en una configuración con todos los valores en rango.
pub fn interpretar(texto: &str) -> Result<Config, String> {
    // Algunos editores dejan la marca BOM al principio; serde_json no la acepta.
    let mut cfg: Config = serde_json::from_str(texto.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
    cfg.normalizar();
    Ok(cfg)
}

/// Guarda una copia de la configuración en otro archivo (exportar).
pub fn exportar(cfg: &Config, destino: &std::path::Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(cfg).map_err(std::io::Error::other)?;
    std::fs::write(destino, json)
}

/// Lee una configuración exportada. No toca `config.json`: quien llama decide si la guarda.
pub fn importar(origen: &std::path::Path) -> Result<Config, String> {
    let texto = std::fs::read_to_string(origen).map_err(|e| e.to_string())?;
    interpretar(&texto)
}

/// Escribe la configuración en disco (la usa la ventana de configuración).
pub fn guardar(cfg: &Config) -> std::io::Result<()> {
    let ruta = ruta();
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(std::io::Error::other)?;
    escribir(&ruta, &json)
}

/// Escribe en un archivo aparte y lo renombra encima del definitivo: si el equipo se apaga a
/// mitad de la escritura, queda el archivo anterior entero y no uno cortado.
fn escribir(ruta: &std::path::Path, json: &str) -> std::io::Result<()> {
    let temporal = ruta.with_extension("json.tmp");
    std::fs::write(&temporal, json)?;
    std::fs::rename(&temporal, ruta)
}

/// Expande `%VAR%` con las variables de entorno. Una variable desconocida se deja tal cual.
pub fn expandir(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(ini) = resto.find('%') {
        salida.push_str(&resto[..ini]);
        let tras = &resto[ini + 1..];
        match tras.find('%') {
            Some(fin) if fin > 0 => {
                let nombre = &tras[..fin];
                match std::env::var(nombre) {
                    Ok(valor) => salida.push_str(&valor),
                    Err(_) => {
                        salida.push('%');
                        salida.push_str(nombre);
                        salida.push('%');
                    }
                }
                resto = &tras[fin + 1..];
            }
            _ => {
                salida.push('%');
                resto = tras;
            }
        }
    }
    salida.push_str(resto);
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expande_variables_conocidas_y_respeta_las_demas() {
        std::env::set_var("ORBYT_PRUEBA", "C:\\x");
        assert_eq!(expandir("%ORBYT_PRUEBA%\\y"), "C:\\x\\y");
        assert_eq!(expandir("%NO_EXISTE_ORBYT%"), "%NO_EXISTE_ORBYT%");
        assert_eq!(expandir("100%"), "100%");
        assert_eq!(expandir("sin variables"), "sin variables");
    }

    #[test]
    fn json_parcial_usa_valores_por_defecto() {
        let cfg: Config = serde_json::from_str(r#"{"accesos":[]}"#).unwrap();
        assert_eq!(cfg.abrir_en, AbrirEn::Cursor);
        assert_eq!(cfg.retardo_ms, 160);
        assert_eq!(cfg.boton, Boton::Central);
        assert!(cfg.oscurecer);
        assert_eq!(cfg.tamano, 1.0);
        assert!(cfg.grupos.is_empty());
        assert_eq!(cfg.modo, Modo::Soltar);
        assert!(!cfg.mostrar_nombres);
        assert!(cfg.pausar_pantalla_completa);
        assert!(cfg.excluidas.is_empty());
        assert_eq!(cfg.opacidad_fondo, 0.88);
        assert_eq!(cfg.color_hover, [72, 64, 96]);
        assert_eq!(cfg.atajo, None);
        assert_eq!(cfg.espera_grupo_ms, 450);
        assert_eq!(cfg.espera_volver_ms, 300);
        assert!(cfg.enfocar_abiertas);
    }

    #[test]
    fn un_grupo_con_programas_solo_sale_en_ellos() {
        let mut g = Grupo {
            nombre: "Editor".into(),
            ..Default::default()
        };
        assert!(g.visible_en(None));
        assert!(g.visible_en(Some("code.exe")));
        g.solo_en = vec!["Code.exe".into(), " blender.exe ".into()];
        assert!(g.visible_en(Some("code.exe")));
        assert!(g.visible_en(Some("blender.exe")));
        assert!(!g.visible_en(Some("chrome.exe")));
        assert!(!g.visible_en(None));
    }

    #[test]
    fn la_lista_de_programas_se_limpia() {
        assert_eq!(
            lista_programas(" Code.exe, blender.exe;;code.exe , \"a b.exe\""),
            ["code.exe", "blender.exe", "a b.exe"]
        );
        assert!(lista_programas("  ").is_empty());
    }

    #[test]
    fn interpretar_normaliza_y_acepta_bom() {
        let cfg =
            interpretar("\u{feff}{\"retardo_ms\":5,\"atajo\":\"  \",\"excluidas\":[\"A.EXE\",\"a.exe\"]}").unwrap();
        assert_eq!(cfg.retardo_ms, RETARDO_MIN);
        assert_eq!(cfg.atajo, None);
        assert_eq!(cfg.excluidas, ["a.exe"]);
        assert!(interpretar("{mal").is_err());
    }

    #[test]
    fn admin_solo_se_escribe_si_esta_activo() {
        let mut a = Acceso {
            nombre: "T".into(),
            destino: "cmd.exe".into(),
            ..Default::default()
        };
        assert!(serde_json::to_value(&a).unwrap().get("admin").is_none());
        a.admin = true;
        assert_eq!(serde_json::to_value(&a).unwrap()["admin"], true);
    }

    #[test]
    fn los_accesos_sueltos_pasan_a_un_grupo_principal() {
        let mut cfg: Config = serde_json::from_str(
            r#"{"modo":"clic","grupos":[{"nombre":"IA"}],"accesos":[{"nombre":"Notas","destino":"notepad.exe"}]}"#,
        )
        .unwrap();
        cfg.normalizar();
        assert_eq!(cfg.modo, Modo::Clic);
        assert!(cfg.accesos.is_empty());
        assert_eq!(cfg.grupos.len(), 2);
        assert_eq!(cfg.grupos[0].nombre, "Principal");
        assert_eq!(cfg.grupos[0].accesos[0].destino, "notepad.exe");
        let json = serde_json::to_value(&cfg).unwrap();
        assert!(json.get("accesos").is_none());
    }

    #[test]
    fn lee_grupos_con_sus_accesos() {
        let cfg: Config = serde_json::from_str(
            r#"{"grupos":[{"nombre":"IA","accesos":[{"nombre":"Claude","destino":"https://claude.ai"}]},{"nombre":"Vacío"}],"accesos":[]}"#,
        )
        .unwrap();
        assert_eq!(cfg.grupos.len(), 2);
        assert_eq!(cfg.grupos[0].accesos[0].nombre, "Claude");
        assert!(cfg.grupos[1].accesos.is_empty());
        assert_eq!(cfg.grupos[1].icono, None);
    }

    #[test]
    fn normalizar_lleva_los_valores_a_su_rango() {
        let mut cfg: Config = serde_json::from_str(
            r#"{"retardo_ms":5,"tamano":9.0,"zona_central":-3,"opacidad_fondo":0.0,"espera_grupo_ms":1,"espera_volver_ms":99999,"accesos":[]}"#,
        )
        .unwrap();
        cfg.normalizar();
        assert_eq!(cfg.retardo_ms, RETARDO_MIN);
        assert_eq!(cfg.tamano, TAMANO_MAX);
        assert_eq!(cfg.zona_central, ZONA_MIN);
        assert_eq!(cfg.opacidad_fondo, OPACIDAD_MIN);
        assert_eq!(cfg.espera_grupo_ms, ESPERA_GRUPO_MIN);
        assert_eq!(cfg.espera_volver_ms, ESPERA_VOLVER_MAX);
    }
}
