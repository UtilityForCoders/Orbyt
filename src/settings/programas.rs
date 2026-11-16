//! Sección Programas: dónde Orbyt deja pasar el botón sin abrir la rueda.

use super::*;

impl Ajustes {
    pub(super) fn programas(&mut self, p: &mut Pintor, x: f32, cw: f32) {
        let mut y = self.titulo(
            p,
            x,
            cw,
            "Programas",
            "Dónde el botón y el atajo funcionan como siempre, sin abrir la rueda.",
        );
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Pausar en pantalla completa",
            "Con un juego o un vídeo a pantalla completa, Orbyt no actúa.",
        );
        let pausar = self.cfg.pausar_pantalla_completa;
        self.interruptor(p, x + cw - 16.0, cy, pausar, Accion::PausarPantallaCompleta);
        y = sig + 14.0;

        y = self.cabecera(p, x, y, cw, "Programas excluidos");
        self.campo_texto(
            p,
            rect(x, y, cw - 120.0, 36.0),
            CampoId::Excluir,
            "Nombre del ejecutable, por ejemplo blender.exe",
            None,
        );
        self.boton(
            p,
            rect(x + cw - 110.0, y, 110.0, 36.0),
            "Excluir",
            Some(glifo::MAS),
            false,
            Accion::AgregarExcluida,
        );
        y += 48.0;
        let excluidos: Vec<(String, Accion)> = self
            .cfg
            .excluidas
            .iter()
            .enumerate()
            .map(|(i, exe)| (exe.clone(), Accion::QuitarExcluida(i)))
            .collect();
        y = if excluidos.is_empty() {
            self.nota(p, x, y, cw, "Ninguno: la rueda sale en todos los programas.")
        } else {
            self.pastillas_programas(p, x, y, cw, &excluidos, true)
        };

        y += 14.0;
        p.texto(
            "Abiertos ahora (clic para excluir)",
            rect(x, y, cw - 40.0, 22.0),
            12.0,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO_2,
        );
        self.icono_boton(
            p,
            x + cw - 14.0,
            y + 11.0,
            glifo::RECARGAR,
            Accion::RecargarAbiertos,
            false,
        );
        y += 28.0;
        let abiertos: Vec<(String, Accion)> = self
            .abiertos
            .iter()
            .enumerate()
            .filter(|(_, exe)| !self.cfg.excluidas.contains(exe))
            .map(|(i, exe)| (exe.clone(), Accion::ExcluirAbierto(i)))
            .collect();
        if abiertos.is_empty() {
            self.nota(p, x, y, cw, "No hay más programas abiertos.");
        } else {
            self.pastillas_programas(p, x, y, cw, &abiertos, false);
        }
    }

    fn nota(&self, p: &mut Pintor, x: f32, y: f32, cw: f32, texto: &str) -> f32 {
        p.texto(
            texto,
            rect(x, y, cw, 24.0),
            12.5,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_3,
        );
        y + 30.0
    }

    /// Una pastilla por programa, en tantas filas como hagan falta. Con `quitar` llevan una equis
    /// (el clic la quita de la lista); sin él, un más (el clic la añade). Devuelve dónde sigue el
    /// contenido.
    fn pastillas_programas(
        &mut self,
        p: &mut Pintor,
        x: f32,
        y: f32,
        ancho: f32,
        items: &[(String, Accion)],
        quitar: bool,
    ) -> f32 {
        let alto = 28.0;
        let (mut cx, mut cy) = (x, y);
        for (texto, accion) in items {
            let w = (p.medir(texto, 12.5, Peso::Medio) + 44.0).min(ancho);
            if cx > x && cx + w > x + ancho {
                cx = x;
                cy += alto + 6.0;
            }
            let r = rect(cx, cy, w, alto);
            let sobre = self.hover == Some(*accion);
            p.rellenar(
                r,
                alto / 2.0,
                if sobre {
                    tema::CONTROL_ACTIVO
                } else {
                    tema::CONTROL_HOVER
                },
            );
            let borde = match (sobre, quitar) {
                (true, true) => tema::PELIGRO,
                (true, false) => tema::BORDE_FUERTE,
                _ => tema::BORDE,
            };
            p.bordear(r, alto / 2.0, borde, 1.0);
            let color = if sobre { tema::TEXTO } else { tema::TEXTO_2 };
            let (g, color_g) = match (quitar, sobre) {
                (true, true) => (glifo::CERRAR, tema::PELIGRO),
                (true, false) => (glifo::CERRAR, tema::TEXTO_3),
                (false, _) => (glifo::MAS, color),
            };
            p.texto(
                texto,
                rect(r.left + 13.0, r.top, w - 44.0, alto),
                12.5,
                Peso::Medio,
                Alin::Izq,
                color,
            );
            p.glifo(g, r.right - 17.0, cy + alto / 2.0, 9.0, color_g);
            self.zona(r, *accion);
            cx += w + 6.0;
        }
        cy + alto + 12.0
    }
}
