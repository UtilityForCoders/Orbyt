//! Controles propios: filas, interruptores, selectores, deslizadores, botones y campos.

use super::*;

impl Ajustes {
    pub(super) fn lateral(&mut self, p: &mut Pintor, h: f32) {
        p.texto(
            "Ajustes",
            rect(22.0, 20.0, 170.0, 30.0),
            20.0,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO,
        );
        let mut y = 70.0;
        for (s, g, nombre) in SECCIONES {
            let r = rect(10.0, y, ANCHO_LATERAL - 20.0, 36.0);
            let activa = self.seccion == s;
            let sobre = self.hover == Some(Accion::Seccion(s));
            if activa {
                p.rellenar(r, 7.0, tema::CONTROL_ACTIVO);
                p.rellenar(rect(r.left, r.top + 10.0, 3.0, 16.0), 1.5, tema::CLARO);
            } else if sobre {
                p.rellenar(r, 7.0, tema::CONTROL);
            }
            let c = if activa { tema::TEXTO } else { tema::TEXTO_2 };
            p.glifo(g, r.left + 22.0, r.top + 18.0, 14.0, c);
            p.texto(
                nombre,
                rect(r.left + 42.0, r.top, 140.0, 36.0),
                13.0,
                if activa { Peso::Semi } else { Peso::Normal },
                Alin::Izq,
                c,
            );
            self.zona(r, Accion::Seccion(s));
            y += 40.0;
        }
        let version = format!("Orbyt {}", env!("CARGO_PKG_VERSION"));
        p.texto(
            &version,
            rect(22.0, h - 40.0, 170.0, 24.0),
            11.5,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_3,
        );
    }

    pub(super) fn titulo(&self, p: &mut Pintor, x: f32, cw: f32, texto: &str, sub: &str) -> f32 {
        p.texto(texto, rect(x, 22.0, cw, 34.0), 24.0, Peso::Semi, Alin::Izq, tema::TEXTO);
        if sub.is_empty() {
            return 76.0;
        }
        p.texto(
            sub,
            rect(x, 56.0, cw, 22.0),
            12.5,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_2,
        );
        92.0
    }

    pub(super) fn cabecera(&self, p: &mut Pintor, x: f32, y: f32, cw: f32, texto: &str) -> f32 {
        p.texto(texto, rect(x, y, cw, 22.0), 12.0, Peso::Semi, Alin::Izq, tema::TEXTO_2);
        y + 28.0
    }

    /// Fila con título y descripción a la izquierda; devuelve el centro vertical del control.
    pub(super) fn fila(&self, p: &mut Pintor, x: f32, y: f32, cw: f32, titulo: &str, desc: &str) -> (f32, f32) {
        let alto = 66.0;
        p.rellenar(rect(x, y, cw, alto - 6.0), 10.0, tema::CONTROL);
        let ancho_txt = (cw - 360.0).max(220.0);
        p.texto(
            titulo,
            rect(x + 16.0, y + 9.0, ancho_txt, 22.0),
            13.5,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO,
        );
        p.texto(
            desc,
            rect(x + 16.0, y + 30.0, ancho_txt, 20.0),
            12.0,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_2,
        );
        (y + (alto - 6.0) / 2.0, y + alto)
    }

    pub(super) fn interruptor(&mut self, p: &mut Pintor, derecha: f32, cy: f32, activo: bool, accion: Accion) {
        let r = rect(derecha - 44.0, cy - 11.0, 44.0, 22.0);
        let sobre = self.hover == Some(accion);
        if activo {
            p.rellenar(r, 11.0, if sobre { rgba(255, 255, 255, 1.0) } else { tema::CLARO });
            p.circulo(r.right - 11.0, cy, 7.0, tema::TEXTO_OSCURO);
        } else {
            p.rellenar(
                r,
                11.0,
                if sobre {
                    tema::CONTROL_ACTIVO
                } else {
                    tema::CONTROL_HOVER
                },
            );
            p.bordear(r, 11.0, tema::BORDE_FUERTE, 1.0);
            p.circulo(r.left + 11.0, cy, 6.0, tema::TEXTO_2);
        }
        self.zona(rect(r.left - 4.0, r.top - 4.0, r.right - r.left + 8.0, 30.0), accion);
    }

    pub(super) fn segmentado(&mut self, p: &mut Pintor, derecha: f32, cy: f32, opciones: &[(&str, Accion, bool)]) {
        let anchos: Vec<f32> = opciones
            .iter()
            .map(|(t, _, _)| p.medir(t, 12.5, Peso::Medio) + 28.0)
            .collect();
        let total: f32 = anchos.iter().sum::<f32>() + 6.0;
        let caja = rect(derecha - total, cy - 16.0, total, 32.0);
        p.rellenar(caja, 9.0, rgba(0, 0, 0, 0.22));
        p.bordear(caja, 9.0, tema::BORDE, 1.0);
        let mut x = caja.left + 3.0;
        for ((texto, accion, sel), ancho) in opciones.iter().zip(anchos) {
            let r = rect(x, caja.top + 3.0, ancho, 26.0);
            let sobre = self.hover == Some(*accion);
            if *sel {
                p.rellenar(r, 7.0, tema::CLARO);
            } else if sobre {
                p.rellenar(r, 7.0, tema::CONTROL_HOVER);
            }
            let c = if *sel {
                tema::TEXTO_OSCURO
            } else if sobre {
                tema::TEXTO
            } else {
                tema::TEXTO_2
            };
            p.texto(texto, r, 12.5, Peso::Medio, Alin::Centro, c);
            self.zona(r, *accion);
            x += ancho;
        }
    }

    pub(super) fn deslizador(&mut self, p: &mut Pintor, derecha: f32, cy: f32, id: Desliz, v: f32, valor: &str) {
        let ancho = 200.0;
        let x = derecha - ancho - 70.0;
        let pista = rect(x, cy - 2.0, ancho, 4.0);
        let v = v.clamp(0.0, 1.0);
        p.rellenar(pista, 2.0, tema::CONTROL_ACTIVO);
        p.rellenar(rect(x, cy - 2.0, ancho * v, 4.0), 2.0, rgba(240, 240, 244, 0.75));
        let activo = self.arrastre == Some(id) || self.hover == Some(Accion::Deslizador(id));
        let kx = x + ancho * v;
        p.circulo(kx, cy + 1.0, 9.0, rgba(0, 0, 0, 0.25));
        p.circulo(kx, cy, if activo { 9.0 } else { 8.0 }, tema::CLARO);
        p.texto(
            valor,
            rect(derecha - 62.0, cy - 12.0, 62.0, 24.0),
            13.0,
            Peso::Semi,
            Alin::Der,
            tema::TEXTO,
        );
        self.pistas.push((id, pista));
        self.zona(rect(x - 10.0, cy - 14.0, ancho + 20.0, 28.0), Accion::Deslizador(id));
    }

    pub(super) fn boton(
        &mut self,
        p: &mut Pintor,
        r: Rect,
        texto: &str,
        g: Option<char>,
        primario: bool,
        accion: Accion,
    ) {
        let sobre = self.hover == Some(accion);
        let (fondo, color): (Color, Color) = match (primario, sobre) {
            (true, false) => (tema::CLARO, tema::TEXTO_OSCURO),
            (true, true) => (rgba(255, 255, 255, 1.0), tema::TEXTO_OSCURO),
            (false, false) => (tema::CONTROL_HOVER, tema::TEXTO),
            (false, true) => (tema::CONTROL_ACTIVO, tema::TEXTO),
        };
        p.rellenar(r, 8.0, fondo);
        if !primario {
            p.bordear(r, 8.0, tema::BORDE, 1.0);
        }
        match g {
            Some(g) if texto.is_empty() => p.glifo(g, (r.left + r.right) / 2.0, (r.top + r.bottom) / 2.0, 13.0, color),
            Some(g) => {
                let ancho = p.medir(texto, 12.5, Peso::Semi) + 22.0;
                let x0 = (r.left + r.right - ancho) / 2.0;
                p.glifo(g, x0 + 7.0, (r.top + r.bottom) / 2.0, 12.0, color);
                p.texto(
                    texto,
                    rect(x0 + 22.0, r.top, ancho, r.bottom - r.top),
                    12.5,
                    Peso::Semi,
                    Alin::Izq,
                    color,
                );
            }
            None => p.texto(texto, r, 12.5, Peso::Semi, Alin::Centro, color),
        }
        self.zona(r, accion);
    }

    pub(super) fn icono_boton(&mut self, p: &mut Pintor, cx: f32, cy: f32, g: char, accion: Accion, peligro: bool) {
        let r = rect(cx - 14.0, cy - 14.0, 28.0, 28.0);
        let sobre = self.hover == Some(accion);
        if sobre {
            p.rellenar(r, 7.0, tema::CONTROL_ACTIVO);
        }
        let c = if sobre && peligro {
            tema::PELIGRO
        } else if sobre {
            tema::TEXTO
        } else {
            tema::TEXTO_2
        };
        p.glifo(g, cx, cy, 12.0, c);
        self.zona(r, accion);
    }

    pub(super) fn campo_texto(&mut self, p: &mut Pintor, r: Rect, id: CampoId, marcador: &str, g: Option<char>) {
        let foco = self.foco == Some(id);
        let sobre = self.hover == Some(Accion::Campo(id));
        p.rellenar(r, 8.0, if foco { rgba(0, 0, 0, 0.30) } else { rgba(0, 0, 0, 0.20) });
        p.bordear(
            r,
            8.0,
            if foco {
                rgba(255, 255, 255, 0.45)
            } else if sobre {
                tema::BORDE_FUERTE
            } else {
                tema::BORDE
            },
            1.0,
        );
        let mut x0 = r.left + 12.0;
        if let Some(g) = g {
            p.glifo(g, r.left + 18.0, (r.top + r.bottom) / 2.0, 12.0, tema::TEXTO_3);
            x0 = r.left + 34.0;
        }
        let interior = rect(x0, r.top, r.right - 10.0 - x0, r.bottom - r.top);
        let campo = self.campos[id as usize].clone();
        let ancho_vis = interior.right - interior.left;
        let cursor_x = p.medir_hasta(&campo.texto, 13.0, Peso::Normal, campo.cursor_utf16());
        // Desplazamiento horizontal para que el cursor siempre se vea.
        let desplaz = (cursor_x - ancho_vis + 2.0).max(0.0);
        p.recortar(interior);
        if campo.texto.is_empty() {
            p.texto(marcador, interior, 13.0, Peso::Normal, Alin::Izq, tema::TEXTO_3);
        } else {
            if foco && campo.todo {
                let ancho = p.medir(&campo.texto, 13.0, Peso::Normal);
                p.rellenar(
                    rect(interior.left - desplaz, r.top + 7.0, ancho, r.bottom - r.top - 14.0),
                    3.0,
                    rgba(120, 150, 255, 0.45),
                );
            }
            let t = rect(interior.left - desplaz, r.top, 10_000.0, r.bottom - r.top);
            p.texto(&campo.texto, t, 13.0, Peso::Normal, Alin::Izq, tema::TEXTO);
        }
        if foco && self.cursor_visible && !campo.todo {
            let cx = interior.left + cursor_x - desplaz;
            p.linea(cx, r.top + 8.0, cx, r.bottom - 8.0, tema::TEXTO, 1.2);
        }
        p.fin_recorte();
        self.campos_r.push((id, interior, desplaz));
        self.zona(r, Accion::Campo(id));
    }

    /// Barra de desplazamiento fina, solo si hace falta.
    pub(super) fn barra(&self, p: &mut Pintor, area: Rect, scroll: f32, max: f32) {
        if max <= 0.0 {
            return;
        }
        let alto = area.bottom - area.top;
        let total = alto + max;
        let largo = (alto * alto / total).max(28.0);
        let y = area.top + (alto - largo) * (scroll / max);
        p.rellenar(rect(area.right - 5.0, y, 3.0, largo), 1.5, rgba(255, 255, 255, 0.22));
    }
}
