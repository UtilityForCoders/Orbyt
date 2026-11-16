//! Campo de texto de una línea, con cursor propio.

/// Texto editable de una línea. El cursor cuenta caracteres, no bytes.
#[derive(Default, Clone)]
pub(super) struct Campo {
    pub(super) texto: String,
    pub(super) cursor: usize,
    /// Ctrl+A: lo próximo que se escriba reemplaza todo.
    pub(super) todo: bool,
}

impl Campo {
    pub(super) fn largo(&self) -> usize {
        self.texto.chars().count()
    }

    pub(super) fn byte(&self, c: usize) -> usize {
        self.texto
            .char_indices()
            .nth(c)
            .map(|(i, _)| i)
            .unwrap_or(self.texto.len())
    }

    /// Posición del cursor en unidades UTF-16 (lo que mide DirectWrite).
    pub(super) fn cursor_utf16(&self) -> usize {
        self.texto.chars().take(self.cursor).map(char::len_utf16).sum()
    }

    pub(super) fn poner(&mut self, s: &str) {
        self.texto = s.to_string();
        self.cursor = self.largo();
        self.todo = false;
    }

    pub(super) fn quitar_seleccion(&mut self) -> bool {
        if self.todo {
            self.texto.clear();
            self.cursor = 0;
            self.todo = false;
            return true;
        }
        false
    }

    pub(super) fn insertar(&mut self, s: &str) {
        self.quitar_seleccion();
        // Una sola línea: los saltos de un texto pegado se vuelven espacios.
        let limpio: String = s
            .chars()
            .map(|c| if c == '\r' || c == '\n' || c == '\t' { ' ' } else { c })
            .collect();
        let b = self.byte(self.cursor);
        self.texto.insert_str(b, &limpio);
        self.cursor += limpio.chars().count();
    }

    pub(super) fn borrar_atras(&mut self) {
        if self.quitar_seleccion() || self.cursor == 0 {
            return;
        }
        let b = self.byte(self.cursor - 1);
        self.texto.remove(b);
        self.cursor -= 1;
    }

    pub(super) fn borrar_delante(&mut self) {
        if self.quitar_seleccion() || self.cursor >= self.largo() {
            return;
        }
        let b = self.byte(self.cursor);
        self.texto.remove(b);
    }

    pub(super) fn mover(&mut self, delta: isize) {
        self.todo = false;
        self.cursor = (self.cursor as isize + delta).clamp(0, self.largo() as isize) as usize;
    }

    pub(super) fn ir(&mut self, posicion: usize) {
        self.todo = false;
        self.cursor = posicion.min(self.largo());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campo_inserta_y_borra_con_acentos() {
        let mut c = Campo::default();
        c.insertar("canción");
        assert_eq!(c.cursor, 7);
        c.mover(-1);
        c.borrar_atras();
        assert_eq!(c.texto, "cancin");
        assert_eq!(c.cursor, 5);
    }

    #[test]
    fn campo_cursor_y_seleccion() {
        let mut c = Campo::default();
        c.poner("hola");
        c.ir(0);
        c.insertar("¡");
        assert_eq!(c.texto, "¡hola");
        assert_eq!(c.cursor_utf16(), 1);
        c.todo = true;
        c.insertar("chau");
        assert_eq!(c.texto, "chau");
        c.ir(usize::MAX);
        c.borrar_delante();
        assert_eq!(c.texto, "chau");
        c.borrar_atras();
        assert_eq!(c.texto, "cha");
    }

    #[test]
    fn campo_pegar_quita_saltos_de_linea() {
        let mut c = Campo::default();
        c.insertar("a\r\nb");
        assert_eq!(c.texto, "a  b");
    }

    #[test]
    fn cursor_utf16_cuenta_pares_sustitutos() {
        let mut c = Campo::default();
        // U+10348 (letra gótica): fuera del plano básico, ocupa dos unidades UTF-16.
        c.poner("a\u{10348}b");
        assert_eq!(c.cursor_utf16(), 4);
    }
}
