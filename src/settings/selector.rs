//! Panel para elegir un ícono: los de Lucide o los que trae un archivo.

use super::*;

impl Ajustes {
    /// Panel para elegir un ícono: los de Lucide o los que trae un archivo.
    pub(super) fn selector_ui(&mut self, p: &mut Pintor, w: f32, h: f32) {
        let Some(modo) = self.selector else { return };
        // Velo sobre todo: un clic fuera del panel lo cierra.
        p.rellenar(rect(0.0, 0.0, w, h), 0.0, rgba(0, 0, 0, 0.55));
        self.zona(rect(0.0, 0.0, w, h), Accion::CerrarSelector);
        let ancho = (w - ANCHO_LATERAL - 80.0).min(760.0);
        let panel = rect(ANCHO_LATERAL + (w - ANCHO_LATERAL - ancho) / 2.0, 40.0, ancho, h - 80.0);
        p.sombra(panel, 14.0, 2.0);
        p.rellenar(panel, 14.0, rgba(30, 30, 34, 0.98));
        p.bordear(panel, 14.0, tema::BORDE_FUERTE, 1.0);
        self.zona(panel, Accion::Bloqueo);

        let (x0, x1) = (panel.left + 20.0, panel.right - 20.0);
        p.texto(
            "Elegir ícono",
            rect(x0, panel.top + 16.0, 200.0, 30.0),
            17.0,
            Peso::Semi,
            Alin::Izq,
            tema::TEXTO,
        );
        self.icono_boton(
            p,
            x1 - 14.0,
            panel.top + 31.0,
            glifo::CERRAR,
            Accion::CerrarSelector,
            false,
        );
        self.segmentado(
            p,
            x1 - 40.0,
            panel.top + 31.0,
            &[
                ("Lucide", Accion::ModoSelector(ModoSel::Lucide), modo == ModoSel::Lucide),
                (
                    "Archivo",
                    Accion::ModoSelector(ModoSel::Archivo),
                    modo == ModoSel::Archivo,
                ),
            ],
        );

        let actual = self.icono_objetivo();
        if actual.is_some() {
            self.boton(
                p,
                rect(x0 + 150.0, panel.top + 15.0, 110.0, 32.0),
                "Automático",
                None,
                false,
                Accion::IconoAuto,
            );
        }
        let y = panel.top + 62.0;
        let total = match modo {
            ModoSel::Lucide => {
                self.campo_texto(
                    p,
                    rect(x0, y, x1 - x0, 36.0),
                    CampoId::BuscarIcono,
                    "Buscar ícono (en inglés: home, mail, music…)",
                    Some(glifo::BUSCAR),
                );
                self.lucide_filtrados.len()
            }
            ModoSel::Archivo => {
                self.campo_texto(
                    p,
                    rect(x0, y, x1 - x0 - 44.0, 36.0),
                    CampoId::Archivo,
                    "Ruta de un .dll, .exe o .ico",
                    None,
                );
                self.boton(
                    p,
                    rect(x1 - 36.0, y, 36.0, 36.0),
                    "",
                    Some(glifo::CARPETA),
                    false,
                    Accion::ExaminarIconos,
                );
                self.archivo_cantidad as usize
            }
        };

        let area = rect(x0, y + 48.0, x1 - x0, panel.bottom - 40.0 - (y + 48.0));
        self.area_sel = area;
        p.rellenar(area, 10.0, rgba(0, 0, 0, 0.22));
        let interior = rect(
            area.left + 6.0,
            area.top + 6.0,
            area.right - area.left - 12.0,
            area.bottom - area.top - 12.0,
        );
        let cols = ((interior.right - interior.left) / CELDA).floor().max(1.0) as usize;
        let filas = total.div_ceil(cols);
        let alto_vis = interior.bottom - interior.top;
        self.max_sel = (filas as f32 * CELDA - alto_vis).max(0.0);
        self.scroll_sel = self.scroll_sel.clamp(0.0, self.max_sel);
        let margen = ((interior.right - interior.left) - cols as f32 * CELDA) / 2.0;

        let mut nombre_hover = None;
        p.recortar(interior);
        if total == 0 {
            let aviso = match modo {
                ModoSel::Lucide => "Ningún ícono coincide",
                ModoSel::Archivo => "Este archivo no tiene íconos (o no existe)",
            };
            p.texto(aviso, interior, 13.0, Peso::Normal, Alin::Centro, tema::TEXTO_2);
        }
        let ruta = self.campos[CampoId::Archivo as usize].texto.trim().to_string();
        let primera = (self.scroll_sel / CELDA) as usize;
        let ultima = ((self.scroll_sel + alto_vis) / CELDA).ceil() as usize + 1;
        for fila in primera..ultima.min(filas) {
            for col in 0..cols {
                let k = fila * cols + col;
                if k >= total {
                    break;
                }
                let cx = interior.left + margen + col as f32 * CELDA;
                let cy = interior.top + fila as f32 * CELDA - self.scroll_sel;
                let celda = rect(cx + 3.0, cy + 3.0, CELDA - 6.0, CELDA - 6.0);
                let (accion, elegido) = match modo {
                    ModoSel::Lucide => {
                        let i = self.lucide_filtrados[k];
                        let n = lucide::nombre_en(i);
                        (
                            Accion::ElegirLucide(i),
                            actual.as_deref() == Some(format!("lucide:{n}").as_str()),
                        )
                    }
                    ModoSel::Archivo => (
                        Accion::ElegirArchivo(k as i32),
                        actual.as_deref() == Some(format!("archivo:{ruta},{k}").as_str()),
                    ),
                };
                let sobre = self.hover == Some(accion);
                if elegido {
                    p.rellenar(celda, 8.0, tema::CONTROL_ACTIVO);
                    p.bordear(celda, 8.0, rgba(255, 255, 255, 0.45), 1.0);
                } else if sobre {
                    p.rellenar(celda, 8.0, tema::CONTROL_HOVER);
                }
                let (mx, my) = (cx + CELDA / 2.0, cy + CELDA / 2.0);
                match modo {
                    ModoSel::Lucide => {
                        let n = lucide::nombre_en(self.lucide_filtrados[k]);
                        let c = if sobre || elegido { tema::TEXTO } else { tema::TEXTO_2 };
                        p.lucide(n, rect(mx - 12.0, my - 12.0, 24.0, 24.0), c);
                        if sobre {
                            nombre_hover = Some(n.to_string());
                        }
                    }
                    ModoSel::Archivo => {
                        let ic = self
                            .iconos_archivo
                            .entry((ruta.clone(), k as i32))
                            .or_insert_with(|| {
                                icons::de_archivo(&ruta, k as i32, 64)
                                    .map(|(an, al, px)| Arc::new(Icono::nuevo(format!("sel:{ruta},{k}"), an, al, px)))
                            })
                            .clone();
                        if let Some(ic) = ic {
                            p.imagen(&ic, rect(mx - 16.0, my - 16.0, 32.0, 32.0), 1.0);
                        }
                        if sobre {
                            nombre_hover = Some(format!("Ícono {k}"));
                        }
                    }
                }
                let arriba = celda.top.max(interior.top);
                let abajo = celda.bottom.min(interior.bottom);
                if abajo > arriba {
                    self.zona(rect(celda.left, arriba, CELDA - 6.0, abajo - arriba), accion);
                }
            }
        }
        p.fin_recorte();
        self.barra(p, interior, self.scroll_sel, self.max_sel);

        let pie = match (nombre_hover, modo) {
            (Some(n), _) => n,
            (None, ModoSel::Lucide) => format!("{total} íconos de Lucide (lucide.dev, licencia ISC)"),
            (None, ModoSel::Archivo) => format!("{total} íconos en el archivo"),
        };
        p.texto(
            &pie,
            rect(x0, panel.bottom - 34.0, x1 - x0, 24.0),
            12.0,
            Peso::Normal,
            Alin::Izq,
            tema::TEXTO_2,
        );
    }
}
