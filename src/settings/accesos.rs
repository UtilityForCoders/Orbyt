//! Sección Accesos: apps instaladas, grupos, la lista del grupo y el editor.

use super::*;

impl Ajustes {
    pub(super) fn accesos(&mut self, p: &mut Pintor, x: f32, cw: f32, h: f32) {
        let y = self.titulo(
            p,
            x,
            cw,
            "Accesos",
            "La rueda muestra tus grupos; cada grupo, sus apps. El orden sigue las agujas del reloj, desde arriba.",
        );
        let sep = 24.0;
        let col = (cw - sep) / 2.0;
        let (xi, xd) = (x, x + col + sep);

        // ── Izquierda: apps instaladas ──
        p.texto(
            "Aplicaciones instaladas",
            rect(xi, y, col - 40.0, 24.0),
            13.5,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO,
        );
        self.icono_boton(p, xi + col - 14.0, y + 12.0, glifo::RECARGAR, Accion::Recargar, false);
        self.campo_texto(
            p,
            rect(xi, y + 34.0, col, 36.0),
            CampoId::Buscar,
            "Buscar aplicación…",
            Some(glifo::BUSCAR),
        );
        let area = rect(xi, y + 80.0, col, h - y - 80.0 - 24.0);
        self.area_apps = area;
        p.rellenar(area, 10.0, rgba(0, 0, 0, 0.18));
        p.bordear(area, 10.0, tema::BORDE, 1.0);
        let interior = rect(
            area.left + 4.0,
            area.top + 4.0,
            area.right - area.left - 8.0,
            area.bottom - area.top - 8.0,
        );
        let alto_vis = interior.bottom - interior.top;
        self.max_apps = (self.filtradas.len() as f32 * FILA_APP - alto_vis).max(0.0);
        self.scroll_apps = self.scroll_apps.clamp(0.0, self.max_apps);
        p.recortar(interior);
        if self.cargando {
            p.texto(
                "Cargando aplicaciones…",
                interior,
                13.0,
                Peso::Normal,
                Alin::Centro,
                tema::TEXTO_2,
            );
        } else if self.filtradas.is_empty() {
            p.texto(
                "Ninguna aplicación coincide",
                interior,
                13.0,
                Peso::Normal,
                Alin::Centro,
                tema::TEXTO_2,
            );
        }
        let primera = (self.scroll_apps / FILA_APP) as usize;
        let ultima = ((self.scroll_apps + alto_vis) / FILA_APP).ceil() as usize + 1;
        for fila in primera..ultima.min(self.filtradas.len()) {
            let i = self.filtradas[fila];
            let ry = interior.top + fila as f32 * FILA_APP - self.scroll_apps;
            let r = rect(interior.left, ry, interior.right - interior.left, FILA_APP);
            let accion = Accion::AgregarApp(i);
            let sobre = self.hover == Some(accion);
            let ya = self.en_rueda(&self.apps[i].destino);
            if sobre {
                p.rellenar(r, 7.0, tema::CONTROL_HOVER);
            }
            if let Some(ic) = self.apps[i].icono.clone() {
                p.imagen(&ic, rect(r.left + 10.0, ry + 8.0, 24.0, 24.0), 1.0);
            }
            let nombre = self.apps[i].nombre.clone();
            p.texto(
                &nombre,
                rect(r.left + 44.0, ry, r.right - r.left - 84.0, FILA_APP),
                13.0,
                Peso::Normal,
                Alin::Izq,
                tema::TEXTO,
            );
            if ya {
                p.glifo(glifo::CHECK, r.right - 20.0, ry + FILA_APP / 2.0, 12.0, tema::TEXTO_3);
            } else if sobre {
                p.glifo(glifo::MAS, r.right - 20.0, ry + FILA_APP / 2.0, 12.0, tema::TEXTO);
            }
            // La zona se recorta al área visible de la lista.
            let rz = rect(
                r.left,
                ry.max(interior.top),
                r.right - r.left,
                (ry + FILA_APP).min(interior.bottom) - ry.max(interior.top),
            );
            self.zona(rz, accion);
        }
        p.fin_recorte();
        self.barra(p, interior, self.scroll_apps, self.max_apps);

        // ── Derecha: grupos, la lista elegida y el editor ──
        let n = self.lista().len();
        let cuenta = match (self.grupo, n) {
            (None, _) => "Sin grupos".to_string(),
            (Some(_), 0) => "En el grupo".to_string(),
            (Some(_), n) if n > RECOMENDADO => format!("En el grupo · {n} (se recomiendan hasta {RECOMENDADO})"),
            (Some(_), n) => format!("En el grupo · {n}"),
        };
        p.texto(
            &cuenta,
            rect(xd, y, col, 24.0),
            13.5,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO,
        );
        let mut yd = self.pestanas_grupos(p, xd, y + 32.0, col);
        if self.grupo.is_some() {
            yd = self.barra_grupo(p, xd, yd, col);
        }
        let alto_editor = 278.0;
        let area = rect(xd, yd, col, (h - yd - 24.0 - alto_editor - 14.0).max(60.0));
        self.area_accesos = area;
        p.rellenar(area, 10.0, rgba(0, 0, 0, 0.18));
        p.bordear(area, 10.0, tema::BORDE, 1.0);
        let interior = rect(
            area.left + 4.0,
            area.top + 4.0,
            area.right - area.left - 8.0,
            area.bottom - area.top - 8.0,
        );
        let alto_vis = interior.bottom - interior.top;
        self.max_accesos = (n as f32 * FILA_ACCESO - alto_vis).max(0.0);
        self.scroll_accesos = self.scroll_accesos.clamp(0.0, self.max_accesos);
        p.recortar(interior);
        if n == 0 {
            let aviso = if self.grupo.is_some() {
                "Grupo vacío: agrega apps desde la izquierda"
            } else {
                "Crea un grupo, o agrega una app y se crea uno"
            };
            p.texto(aviso, interior, 13.0, Peso::Normal, Alin::Centro, tema::TEXTO_2);
        }
        for i in 0..n {
            let ry = interior.top + i as f32 * FILA_ACCESO - self.scroll_accesos;
            if ry + FILA_ACCESO < interior.top || ry > interior.bottom {
                continue;
            }
            let r = rect(interior.left, ry, interior.right - interior.left, FILA_ACCESO);
            let elegida = self.seleccion == Some(i);
            let sobre = matches!(self.hover, Some(Accion::Elegir(j) | Accion::Subir(j) | Accion::Bajar(j) | Accion::Quitar(j)) if j == i);
            if elegida {
                p.rellenar(r, 7.0, tema::CONTROL_ACTIVO);
            } else if sobre {
                p.rellenar(r, 7.0, tema::CONTROL_HOVER);
            }
            let rz = rect(
                r.left,
                ry.max(interior.top),
                r.right - r.left,
                (ry + FILA_ACCESO).min(interior.bottom) - ry.max(interior.top),
            );
            self.zona(rz, Accion::Elegir(i));
            let acceso = self.lista()[i].clone();
            p.texto(
                &format!("{}", i + 1),
                rect(r.left + 6.0, ry, 18.0, FILA_ACCESO),
                11.5,
                Peso::Normal,
                Alin::Centro,
                tema::TEXTO_3,
            );
            match icons::visual(&acceso) {
                Some(v) => p.visual(&v, rect(r.left + 30.0, ry + 9.0, 26.0, 26.0), tema::TEXTO),
                None => {
                    let ri = rect(r.left + 30.0, ry + 9.0, 26.0, 26.0);
                    p.rellenar(ri, 7.0, tema::CONTROL_ACTIVO);
                    let ini: String = acceso
                        .nombre
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().collect())
                        .unwrap_or_default();
                    p.texto(&ini, ri, 12.0, Peso::Semi, Alin::Centro, tema::TEXTO);
                }
            }
            let ancho_txt = r.right - r.left - 66.0 - if sobre || elegida { 100.0 } else { 0.0 };
            p.texto(
                &acceso.nombre,
                rect(r.left + 66.0, ry, ancho_txt, FILA_ACCESO),
                13.0,
                Peso::Medio,
                Alin::Izq,
                tema::TEXTO,
            );
            if acceso.admin {
                // Marca discreta de "se abre como administrador".
                if let Some(escudo) = lucide::nombre("shield") {
                    let xm = r.right - 22.0 - if sobre || elegida { 100.0 } else { 0.0 };
                    p.lucide(escudo, rect(xm, ry + 15.0, 14.0, 14.0), tema::TEXTO_3);
                }
            }
            if sobre || elegida {
                let cy = ry + FILA_ACCESO / 2.0;
                self.icono_boton(p, r.right - 86.0, cy, glifo::ARRIBA, Accion::Subir(i), false);
                self.icono_boton(p, r.right - 54.0, cy, glifo::ABAJO, Accion::Bajar(i), false);
                self.icono_boton(p, r.right - 20.0, cy, glifo::BORRAR, Accion::Quitar(i), true);
            }
        }
        // Al arrastrar un acceso, una línea marca dónde quedaría.
        if let Some(Arrastrando {
            que: Que::Fila(desde),
            destino,
            activo: true,
            ..
        }) = self.arrastrando
        {
            let hueco = destino + usize::from(destino > desde);
            let ly = interior.top + hueco as f32 * FILA_ACCESO - self.scroll_accesos;
            p.linea(interior.left + 6.0, ly, interior.right - 6.0, ly, tema::CLARO, 2.0);
        }
        p.fin_recorte();
        self.barra(p, interior, self.scroll_accesos, self.max_accesos);

        // Editor del acceso elegido (o de uno nuevo).
        let ed = rect(xd, area.bottom + 14.0, col, alto_editor);
        p.rellenar(ed, 10.0, tema::CONTROL);
        p.bordear(ed, 10.0, tema::BORDE, 1.0);
        let titulo = if self.seleccion.is_some() {
            "Editar acceso"
        } else {
            "Nuevo acceso"
        };
        // Casilla "Como administrador", a la derecha del título.
        let etiqueta = "Como administrador";
        let ancho_et = p.medir(etiqueta, 12.0, Peso::Normal);
        let admin = self.admin_editor;
        let sobre = self.hover == Some(Accion::Admin);
        let casilla = rect(ed.right - 16.0 - ancho_et - 24.0, ed.top + 14.0, 16.0, 16.0);
        p.rellenar(casilla, 4.0, if admin { tema::CLARO } else { rgba(0, 0, 0, 0.25) });
        p.bordear(casilla, 4.0, if sobre { tema::BORDE_FUERTE } else { tema::BORDE }, 1.0);
        if admin {
            p.glifo(
                glifo::CHECK,
                casilla.left + 8.0,
                casilla.top + 8.0,
                9.0,
                tema::TEXTO_OSCURO,
            );
        }
        p.texto(
            etiqueta,
            rect(casilla.right + 8.0, ed.top + 10.0, ancho_et + 4.0, 24.0),
            12.0,
            Peso::Normal,
            Alin::Izq,
            if sobre { tema::TEXTO } else { tema::TEXTO_2 },
        );
        self.zona(
            rect(casilla.left - 4.0, ed.top + 8.0, ancho_et + 36.0, 28.0),
            Accion::Admin,
        );
        p.texto(
            titulo,
            rect(ed.left + 16.0, ed.top + 10.0, col - 32.0 - ancho_et - 40.0, 24.0),
            13.0,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO,
        );
        let et = 86.0;
        let xc = ed.left + 16.0 + et;
        let wc = col - 32.0 - et;
        let mut fy = ed.top + 42.0;
        for (id, etiqueta, marcador) in [
            (CampoId::Nombre, "Nombre", "Cómo se ve en la rueda"),
            (
                CampoId::Destino,
                "Destino",
                "Programa, carpeta, archivo, URL o ms-settings:",
            ),
            (CampoId::Args, "Argumentos", "Opcional"),
        ] {
            p.texto(
                etiqueta,
                rect(ed.left + 16.0, fy, et, 34.0),
                12.5,
                Peso::Normal,
                Alin::Izq,
                tema::TEXTO_2,
            );
            let ancho = if id == CampoId::Destino { wc - 42.0 } else { wc };
            self.campo_texto(p, rect(xc, fy, ancho, 34.0), id, marcador, None);
            if id == CampoId::Destino {
                self.boton(
                    p,
                    rect(xc + wc - 34.0, fy, 34.0, 34.0),
                    "",
                    Some(glifo::CARPETA),
                    false,
                    Accion::Examinar,
                );
            }
            fy += 42.0;
        }
        // Ícono: vista previa, elegir uno propio o volver al automático.
        p.texto(
            "Ícono",
            rect(ed.left + 16.0, fy, et, 34.0),
            12.5,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_2,
        );
        let caja = rect(xc, fy, 34.0, 34.0);
        p.rellenar(caja, 8.0, rgba(0, 0, 0, 0.25));
        p.bordear(caja, 8.0, tema::BORDE, 1.0);
        let destino = self.campos[CampoId::Destino as usize].texto.clone();
        let vista = match &self.icono_editor {
            Some(i) => icons::visual_propio(i),
            None => icons::automatico(&destino),
        };
        if let Some(v) = vista {
            p.visual(&v, rect(xc + 6.0, fy + 6.0, 22.0, 22.0), tema::TEXTO);
        }
        self.boton(
            p,
            rect(xc + 44.0, fy, 110.0, 34.0),
            "Elegir…",
            None,
            false,
            Accion::AbrirSelector,
        );
        // Con un ícono propio, un botón para volver al automático.
        if self.icono_editor.is_some() {
            self.boton(
                p,
                rect(xc + 162.0, fy, (wc - 162.0).min(116.0), 34.0),
                "Automático",
                None,
                false,
                Accion::IconoAuto,
            );
        }
        fy += 42.0;
        let wb = (wc - 10.0) / 2.0;
        self.boton(
            p,
            rect(xc, fy + 4.0, wb, 34.0),
            "Guardar",
            Some(glifo::CHECK),
            true,
            Accion::Guardar,
        );
        self.boton(
            p,
            rect(xc + wb + 10.0, fy + 4.0, wb, 34.0),
            "Nuevo",
            Some(glifo::MAS),
            false,
            Accion::Nuevo,
        );
    }

    /// Pastillas para elegir qué grupo se edita, y "+ Grupo".
    /// Si no caben en una fila, siguen en la de abajo. Devuelve dónde sigue el contenido.
    pub(super) fn pestanas_grupos(&mut self, p: &mut Pintor, x: f32, y: f32, ancho: f32) -> f32 {
        let alto = 28.0;
        let mut items: Vec<(String, Accion, bool)> = Vec::new();
        for (i, g) in self.cfg.grupos.iter().enumerate() {
            let nombre = if g.nombre.trim().is_empty() {
                "Sin nombre".to_string()
            } else {
                g.nombre.clone()
            };
            items.push((nombre, Accion::Contexto(Some(i)), self.grupo == Some(i)));
        }
        let (mut cx, mut cy) = (x, y);
        for (texto, accion, sel) in items {
            let w = (p.medir(&texto, 12.5, Peso::Medio) + 26.0).min(ancho);
            if cx > x && cx + w > x + ancho {
                cx = x;
                cy += alto + 6.0;
            }
            let r = rect(cx, cy, w, alto);
            let sobre = self.hover == Some(accion);
            if sel {
                p.rellenar(r, alto / 2.0, tema::CLARO);
            } else {
                p.rellenar(
                    r,
                    alto / 2.0,
                    if sobre {
                        tema::CONTROL_ACTIVO
                    } else {
                        tema::CONTROL_HOVER
                    },
                );
                p.bordear(r, alto / 2.0, tema::BORDE, 1.0);
            }
            let c = if sel {
                tema::TEXTO_OSCURO
            } else if sobre {
                tema::TEXTO
            } else {
                tema::TEXTO_2
            };
            p.texto(
                &texto,
                rect(r.left + 13.0, r.top, w - 26.0, alto),
                12.5,
                Peso::Medio,
                Alin::Centro,
                c,
            );
            if let Accion::Contexto(Some(g)) = accion {
                self.pastillas.push((g, r));
                // Al arrastrar un grupo, se marca la pastilla cuyo sitio ocuparía.
                let destino = matches!(
                    self.arrastrando,
                    Some(Arrastrando { que: Que::Grupo(desde), destino, activo: true, .. }) if destino == g && desde != g
                );
                if destino {
                    p.bordear(r, alto / 2.0, tema::CLARO, 1.5);
                }
            }
            self.zona(r, accion);
            cx += w + 6.0;
        }
        // "+ Grupo"
        let w = 84.0;
        if cx > x && cx + w > x + ancho {
            cx = x;
            cy += alto + 6.0;
        }
        let r = rect(cx, cy, w, alto);
        let sobre = self.hover == Some(Accion::NuevoGrupo);
        p.rellenar(
            r,
            alto / 2.0,
            if sobre {
                tema::CONTROL_ACTIVO
            } else {
                rgba(0, 0, 0, 0.0)
            },
        );
        p.bordear(r, alto / 2.0, if sobre { tema::BORDE_FUERTE } else { tema::BORDE }, 1.0);
        let c = if sobre { tema::TEXTO } else { tema::TEXTO_2 };
        p.glifo(glifo::MAS, r.left + 18.0, cy + alto / 2.0, 10.0, c);
        p.texto(
            "Grupo",
            rect(r.left + 30.0, cy, w - 38.0, alto),
            12.5,
            Peso::Medio,
            Alin::Izq,
            c,
        );
        self.zona(r, Accion::NuevoGrupo);
        cy + alto + 12.0
    }

    /// Ícono, nombre, orden y borrar del grupo que se está editando.
    pub(super) fn barra_grupo(&mut self, p: &mut Pintor, x: f32, y: f32, ancho: f32) -> f32 {
        let Some(g) = self.grupo.filter(|&g| g < self.cfg.grupos.len()) else {
            return y;
        };
        let alto = 34.0;
        // Ícono: clic para elegir otro.
        let caja = rect(x, y, alto, alto);
        let sobre = self.hover == Some(Accion::IconoGrupo);
        p.rellenar(
            caja,
            8.0,
            if sobre {
                tema::CONTROL_ACTIVO
            } else {
                rgba(0, 0, 0, 0.25)
            },
        );
        p.bordear(caja, 8.0, if sobre { tema::BORDE_FUERTE } else { tema::BORDE }, 1.0);
        let visual = self.cfg.grupos[g].icono.as_deref().and_then(icons::visual_propio);
        match visual {
            Some(v) => p.visual(&v, rect(x + 7.0, y + 7.0, 20.0, 20.0), tema::TEXTO),
            None => {
                if let Some(n) = lucide::nombre("folder") {
                    p.lucide(n, rect(x + 8.0, y + 8.0, 18.0, 18.0), tema::TEXTO_2);
                }
            }
        }
        self.zona(caja, Accion::IconoGrupo);

        let confirmar = self.confirmar_quitar;
        let ancho_botones = if confirmar { 64.0 + 112.0 } else { 100.0 };
        self.campo_texto(
            p,
            rect(x + alto + 8.0, y, ancho - alto - 8.0 - ancho_botones, alto),
            CampoId::NombreGrupo,
            "Nombre del grupo",
            None,
        );
        let cy = y + alto / 2.0;
        let derecha = x + ancho;
        if confirmar {
            self.icono_boton(
                p,
                derecha - 112.0 - 46.0,
                cy,
                glifo::IZQUIERDA,
                Accion::GrupoIzquierda,
                false,
            );
            self.icono_boton(
                p,
                derecha - 112.0 - 14.0,
                cy,
                glifo::DERECHA,
                Accion::GrupoDerecha,
                false,
            );
            let r = rect(derecha - 104.0, y + 2.0, 104.0, alto - 4.0);
            let sobre = self.hover == Some(Accion::QuitarGrupo);
            p.rellenar(r, 8.0, if sobre { tema::PELIGRO } else { rgba(0, 0, 0, 0.25) });
            p.bordear(r, 8.0, tema::PELIGRO, 1.0);
            p.texto(
                "¿Quitar grupo?",
                r,
                12.0,
                Peso::Semi,
                Alin::Centro,
                if sobre { tema::TEXTO_OSCURO } else { tema::PELIGRO },
            );
            self.zona(r, Accion::QuitarGrupo);
        } else {
            self.icono_boton(p, derecha - 82.0, cy, glifo::IZQUIERDA, Accion::GrupoIzquierda, false);
            self.icono_boton(p, derecha - 50.0, cy, glifo::DERECHA, Accion::GrupoDerecha, false);
            self.icono_boton(p, derecha - 16.0, cy, glifo::BORRAR, Accion::QuitarGrupo, true);
        }
        // Programas en los que aparece el grupo: vacío, en todos.
        let y2 = y + alto + 8.0;
        p.texto(
            "Solo en",
            rect(x, y2, 56.0, alto),
            12.5,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_2,
        );
        self.campo_texto(
            p,
            rect(x + 60.0, y2, ancho - 60.0, alto),
            CampoId::SoloEn,
            "Todos los programas (o code.exe, blender.exe)",
            None,
        );
        y2 + alto + 12.0
    }
}
