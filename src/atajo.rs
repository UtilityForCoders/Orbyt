//! Atajo de teclado que abre la rueda: leerlo de `config.json` (`Ctrl+Alt+Espacio`) y escribirlo
//! a partir de las teclas pulsadas en los ajustes. Sin Win32, para poder probarlo.

/// Modificadores, con los mismos valores que `MOD_ALT`, `MOD_CONTROL`, `MOD_SHIFT` y `MOD_WIN`.
pub const ALT: u32 = 1;
pub const CTRL: u32 = 2;
pub const MAYUS: u32 = 4;
pub const WIN: u32 = 8;

/// Teclas con nombre: (código virtual, nombre que se escribe, otros nombres que se aceptan).
const TECLAS: &[(u32, &str, &[&str])] = &[
    (0x20, "Espacio", &["space"]),
    (0x09, "Tab", &["tabulador"]),
    (0x0D, "Intro", &["enter", "return"]),
    (0x08, "Retroceso", &["backspace"]),
    (0x2D, "Insert", &["ins", "insertar"]),
    (0x2E, "Supr", &["del", "delete", "suprimir"]),
    (0x24, "Inicio", &["home"]),
    (0x23, "Fin", &["end"]),
    (0x21, "RePág", &["repag", "pageup", "pgup"]),
    (0x22, "AvPág", &["avpag", "pagedown", "pgdn"]),
    (0x25, "Izquierda", &["left"]),
    (0x26, "Arriba", &["up"]),
    (0x27, "Derecha", &["right"]),
    (0x28, "Abajo", &["down"]),
    (0x13, "Pausa", &["pause"]),
    (0x91, "BloqDespl", &["scrolllock"]),
];

/// Modificadores sueltos: no sirven como tecla del atajo.
const SOLO_MODIFICADORES: &[u32] = &[0x10, 0x11, 0x12, 0x5B, 0x5C, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5];

fn nombre_tecla(vk: u32) -> Option<String> {
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk).map(String::from),
        0x70..=0x87 => Some(format!("F{}", vk - 0x6F)),
        0x60..=0x69 => Some(format!("Num{}", vk - 0x60)),
        _ => TECLAS.iter().find(|t| t.0 == vk).map(|t| t.1.to_string()),
    }
}

fn tecla_de(nombre: &str) -> Option<u32> {
    let n = nombre.to_lowercase();
    let mut letras = n.chars();
    if let (Some(c), None) = (letras.next(), letras.next()) {
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase() as u32);
        }
    }
    if let Some(num) = n.strip_prefix('f').and_then(|x| x.parse::<u32>().ok()) {
        return (1..=24).contains(&num).then_some(0x6F + num);
    }
    if let Some(num) = n.strip_prefix("num").and_then(|x| x.parse::<u32>().ok()) {
        return (num <= 9).then_some(0x60 + num);
    }
    TECLAS
        .iter()
        .find(|t| t.1.to_lowercase() == n || t.2.contains(&n.as_str()))
        .map(|t| t.0)
}

/// ¿Vale esta combinación como atajo global? Hace falta una tecla que no sea un modificador y,
/// salvo en las teclas de función, al menos Ctrl, Alt o Win: un atajo de una sola letra (o con
/// solo Mayús) dejaría de poder escribirse en todo el sistema.
pub fn valido(mods: u32, vk: u32) -> bool {
    if SOLO_MODIFICADORES.contains(&vk) || nombre_tecla(vk).is_none() {
        return false;
    }
    (0x70..=0x87).contains(&vk) || mods & (CTRL | ALT | WIN) != 0
}

/// Texto del atajo, como se guarda y se muestra: `Ctrl+Alt+Espacio`.
pub fn escribir(mods: u32, vk: u32) -> Option<String> {
    if !valido(mods, vk) {
        return None;
    }
    let mut partes: Vec<String> = Vec::new();
    for (m, nombre) in [(CTRL, "Ctrl"), (ALT, "Alt"), (MAYUS, "Mayús"), (WIN, "Win")] {
        if mods & m != 0 {
            partes.push(nombre.to_string());
        }
    }
    partes.push(nombre_tecla(vk)?);
    Some(partes.join("+"))
}

/// Modificadores y tecla de un atajo escrito; `None` si no se entiende o no vale como atajo.
pub fn leer(texto: &str) -> Option<(u32, u32)> {
    let mut mods = 0;
    let mut tecla = None;
    for parte in texto.split('+').map(str::trim) {
        match parte.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= CTRL,
            "alt" => mods |= ALT,
            "mayús" | "mayus" | "shift" => mods |= MAYUS,
            "win" | "windows" => mods |= WIN,
            _ if tecla.is_none() => tecla = Some(tecla_de(parte)?),
            _ => return None,
        }
    }
    let vk = tecla?;
    valido(mods, vk).then_some((mods, vk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_y_escribe_el_mismo_atajo() {
        for (mods, vk) in [
            (CTRL | ALT, 0x20),
            (WIN, 0x41),
            (CTRL | MAYUS, 0x39),
            (0, 0x77),
            (ALT, 0x61),
            (CTRL, 0x22),
        ] {
            let texto = escribir(mods, vk).unwrap();
            assert_eq!(leer(&texto), Some((mods, vk)), "{texto}");
        }
        assert_eq!(escribir(CTRL | ALT, 0x20).as_deref(), Some("Ctrl+Alt+Espacio"));
    }

    #[test]
    fn acepta_nombres_en_ingles_y_sin_acentos() {
        assert_eq!(leer("ctrl + shift + space"), Some((CTRL | MAYUS, 0x20)));
        assert_eq!(leer("Control+Mayus+f12"), Some((CTRL | MAYUS, 0x7B)));
        assert_eq!(leer("Alt+PageDown"), Some((ALT, 0x22)));
    }

    #[test]
    fn rechaza_lo_que_no_sirve_de_atajo() {
        assert_eq!(leer(""), None);
        assert_eq!(leer("Ctrl+Alt"), None);
        assert_eq!(leer("A"), None);
        assert_eq!(leer("Mayús+A"), None);
        assert_eq!(leer("Ctrl+A+B"), None);
        assert_eq!(leer("Ctrl+Nada"), None);
        assert!(!valido(CTRL, 0x11));
        assert!(valido(0, 0x70));
    }
}
