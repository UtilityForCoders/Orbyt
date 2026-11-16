//! Secciones General, Activación, Apariencia y Acerca de.

use super::*;

impl Ajustes {
    pub(super) fn general(&mut self, p: &mut Pintor, x: f32, cw: f32) {
        let mut y = self.titulo(p, x, cw, "General", "");
        y = self.cabecera(p, x, y, cw, "Rueda");
        let (cy, sig) = self.fila(p, x, y, cw, "Posición", "Dónde aparece la rueda al mantener el botón.");
        let a = self.cfg.abrir_en;
        self.segmentado(
            p,
            x + cw - 16.0,
            cy,
            &[
                (
                    "Bajo el puntero",
                    Accion::AbrirEn(AbrirEn::Cursor),
                    a == AbrirEn::Cursor,
                ),
                (
                    "Centro de la pantalla",
                    Accion::AbrirEn(AbrirEn::Centro),
                    a == AbrirEn::Centro,
                ),
            ],
        );
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Retardo",
            "Cuánto mantener el botón antes de que salga la rueda.",
        );
        let v = (self.cfg.retardo_ms - config::RETARDO_MIN) as f32 / (config::RETARDO_MAX - config::RETARDO_MIN) as f32;
        let texto = format!("{} ms", self.cfg.retardo_ms);
        self.deslizador(p, x + cw - 16.0, cy, Desliz::Retardo, v, &texto);
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Abrir un grupo",
            "Al elegir soltando: cuánto apuntar a un grupo para que se abra.",
        );
        let v = (self.cfg.espera_grupo_ms - config::ESPERA_GRUPO_MIN) as f32
            / (config::ESPERA_GRUPO_MAX - config::ESPERA_GRUPO_MIN) as f32;
        let texto = format!("{} ms", self.cfg.espera_grupo_ms);
        self.deslizador(p, x + cw - 16.0, cy, Desliz::EsperaGrupo, v, &texto);
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Volver atrás",
            "Al elegir soltando: cuánto quedarse en el centro para volver.",
        );
        let v = (self.cfg.espera_volver_ms - config::ESPERA_VOLVER_MIN) as f32
            / (config::ESPERA_VOLVER_MAX - config::ESPERA_VOLVER_MIN) as f32;
        let texto = format!("{} ms", self.cfg.espera_volver_ms);
        self.deslizador(p, x + cw - 16.0, cy, Desliz::EsperaVolver, v, &texto);
        y = sig + 14.0;
        y = self.cabecera(p, x, y, cw, "Sistema");
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Iniciar con Windows",
            "Orbyt se abre solo al iniciar sesión y queda en la bandeja.",
        );
        let activo = self.inicio_windows;
        self.interruptor(p, x + cw - 16.0, cy, activo, Accion::InicioWindows);
        y = sig;
        let (cy, _) = self.fila(
            p,
            x,
            y,
            cw,
            "Ir a la ventana abierta",
            "Si el programa ya está abierto, se trae al frente.",
        );
        let enfocar = self.cfg.enfocar_abiertas;
        self.interruptor(p, x + cw - 16.0, cy, enfocar, Accion::EnfocarAbiertas);
    }

    pub(super) fn activacion(&mut self, p: &mut Pintor, x: f32, cw: f32) {
        let mut y = self.titulo(p, x, cw, "Activación", "");
        y = self.cabecera(p, x, y, cw, "Ratón");
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Botón",
            "Qué botón abre la rueda. Los laterales suelen estar libres.",
        );
        let b = self.cfg.boton;
        self.segmentado(
            p,
            x + cw - 16.0,
            cy,
            &[
                ("Central", Accion::Boton(Boton::Central), b == Boton::Central),
                ("Atrás", Accion::Boton(Boton::Atras), b == Boton::Atras),
                ("Adelante", Accion::Boton(Boton::Adelante), b == Boton::Adelante),
            ],
        );
        y = sig;
        let m = self.cfg.modo;
        let desc = match m {
            Modo::Soltar => "Apuntas y sueltas; un grupo se abre al apuntarlo un momento.",
            Modo::Clic => "El botón abre la rueda; eliges con clic izquierdo.",
        };
        let (cy, sig) = self.fila(p, x, y, cw, "Elegir con", desc);
        self.segmentado(
            p,
            x + cw - 16.0,
            cy,
            &[
                ("Soltar el botón", Accion::Modo(Modo::Soltar), m == Modo::Soltar),
                ("Clic", Accion::Modo(Modo::Clic), m == Modo::Clic),
            ],
        );
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Zona central",
            "Círculo del centro: cancela o vuelve atrás.",
        );
        let v = (self.cfg.zona_central - config::ZONA_MIN) / (config::ZONA_MAX - config::ZONA_MIN);
        let texto = format!("{} px", self.cfg.zona_central as i32);
        self.deslizador(p, x + cw - 16.0, cy, Desliz::Zona, v, &texto);
        y = sig + 14.0;
        y = self.cabecera(p, x, y, cw, "Teclado");
        let capturando = self.capturando;
        let desc = if capturando {
            "Pulsa la combinación. Escape cancela."
        } else {
            "Abre la rueda sin ratón; se elige con clic o con el teclado."
        };
        let (cy, sig) = self.fila(p, x, y, cw, "Atajo", desc);
        let derecha = x + cw - 16.0;
        let texto = match (&self.cfg.atajo, capturando) {
            (_, true) => "Pulsa las teclas…".to_string(),
            (Some(a), false) => a.clone(),
            (None, false) => "Ninguno".to_string(),
        };
        let mut xb = derecha;
        if self.cfg.atajo.is_some() && !capturando {
            self.icono_boton(p, derecha - 14.0, cy, glifo::CERRAR, Accion::QuitarAtajo, true);
            xb -= 36.0;
        }
        let ancho = (p.medir(&texto, 12.5, Peso::Semi) + 32.0).max(110.0);
        self.boton(
            p,
            rect(xb - ancho, cy - 16.0, ancho, 32.0),
            &texto,
            None,
            capturando,
            Accion::CapturarAtajo,
        );
        y = sig;
        let (cy, _) = self.fila(
            p,
            x,
            y,
            cw,
            "Con la rueda abierta",
            "1 a 9, o flechas e Intro, eligen; Retroceso vuelve; Escape cierra.",
        );
        p.texto(
            "Siempre activo",
            rect(x + cw - 216.0, cy - 12.0, 200.0, 24.0),
            12.5,
            Peso::Normal,
            Alin::Der,
            tema::TEXTO_3,
        );
    }

    pub(super) fn apariencia(&mut self, p: &mut Pintor, x: f32, cw: f32, h: f32) {
        let mut y = self.titulo(p, x, cw, "Apariencia", "");
        y = self.cabecera(p, x, y, cw, "Rueda");
        let (cy, sig) = self.fila(p, x, y, cw, "Tamaño", "Escala de los botones orbitales y del centro.");
        let v = (self.cfg.tamano - config::TAMANO_MIN) / (config::TAMANO_MAX - config::TAMANO_MIN);
        let texto = format!("{} %", (self.cfg.tamano * 100.0).round() as i32);
        self.deslizador(p, x + cw - 16.0, cy, Desliz::Tamano, v, &texto);
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Oscurecer el fondo",
            "Atenúa un poco la pantalla mientras la rueda está abierta.",
        );
        let o = self.cfg.oscurecer;
        self.interruptor(p, x + cw - 16.0, cy, o, Accion::Oscurecer);
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Mostrar nombre seleccionado",
            "Muestra junto a la rueda el nombre del acceso al que apuntas.",
        );
        let mn = self.cfg.mostrar_nombres;
        self.interruptor(p, x + cw - 16.0, cy, mn, Accion::MostrarNombres);
        y = sig;
        let (cy, sig) = self.fila(p, x, y, cw, "Color de resaltado", "Color del botón al que apuntas.");
        let derecha = x + cw - 16.0;
        self.boton(
            p,
            rect(derecha - 70.0, cy - 16.0, 70.0, 32.0),
            "Otro…",
            None,
            false,
            Accion::ColorOtro,
        );
        let actual = self.cfg.color_hover;
        let mut cx = derecha - 70.0 - 26.0;
        for (i, c) in COLORES.iter().enumerate().rev() {
            let accion = Accion::Color(i);
            if *c == actual {
                p.borde_circulo(cx, cy, 14.0, tema::CLARO, 1.5);
            } else if self.hover == Some(accion) {
                p.borde_circulo(cx, cy, 14.0, tema::BORDE_FUERTE, 1.5);
            }
            p.circulo(cx, cy, 10.0, rgba(c[0], c[1], c[2], 1.0));
            self.zona(rect(cx - 15.0, cy - 15.0, 30.0, 30.0), accion);
            cx -= 32.0;
        }
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Opacidad de los botones",
            "Nivel de transparencia de los botones orbitales.",
        );
        let v_op = (self.cfg.opacidad_fondo - config::OPACIDAD_MIN) / (config::OPACIDAD_MAX - config::OPACIDAD_MIN);
        let texto_op = format!("{} %", (self.cfg.opacidad_fondo * 100.0).round() as i32);
        self.deslizador(p, x + cw - 16.0, cy, Desliz::OpacidadFondo, v_op, &texto_op);
        y = sig + 14.0;

        // Vista previa en vivo, con los accesos y el tamaño actuales.
        y = self.cabecera(p, x, y, cw, "Vista previa");
        let caja = rect(x, y, cw, (h - y - 24.0).max(120.0));
        p.rellenar(caja, 12.0, rgba(0, 0, 0, 0.25));
        p.bordear(caja, 12.0, tema::BORDE, 1.0);
        let baldosas = render::baldosas_raiz(&self.cfg);
        // Cabe en la caja: si la rueda es más grande que el espacio, se reduce solo en la vista.
        let alcance = crate::wheel::alcance(baldosas.len()) * self.cfg.tamano;
        let lado = (caja.right - caja.left).min(caja.bottom - caja.top);
        let ajuste = (lado / 2.0 / alcance).min(1.0);
        let escena = Escena {
            centro: ((caja.left + caja.right) / 2.0, (caja.top + caja.bottom) / 2.0),
            escala: self.cfg.tamano * ajuste,
            baldosas: &baldosas,
            grupo: None,
            seleccion: (!baldosas.is_empty()).then_some(0),
            progreso: 1.0,
            progreso_grupo: 0.45,
            radio_central: self.cfg.zona_central * self.cfg.tamano * ajuste,
            mostrar_nombres: self.cfg.mostrar_nombres,
            color_hover: self.cfg.color_hover,
            opacidad_fondo: self.cfg.opacidad_fondo,
        };
        p.recortar(caja);
        render::pintar_rueda(p, &escena);
        p.fin_recorte();
    }

    pub(super) fn acerca(&mut self, p: &mut Pintor, x: f32, cw: f32) {
        let mut y = self.titulo(p, x, cw, "Acerca de", "");
        use actualizacion::Estado;
        let buscar = Some(("Buscar actualización".to_string(), Accion::BuscarActualizacion));
        let (desc, boton) = match &self.actualizacion {
            Estado::SinComprobar => ("Lanzador radial para Windows, en Rust nativo.".to_string(), buscar),
            Estado::Buscando => ("Buscando una versión nueva…".to_string(), None),
            Estado::AlDia => ("Tienes la última versión.".to_string(), buscar),
            Estado::Disponible { version, .. } => (
                format!("Hay una versión nueva: {version}."),
                Some(("Descargar".to_string(), Accion::AbrirDescarga)),
            ),
            Estado::SinRespuesta => ("No se pudo comprobar. Revisa la conexión.".to_string(), buscar),
        };
        let titulo = format!("Versión {}", env!("CARGO_PKG_VERSION"));
        let (cy, sig) = self.fila(p, x, y, cw, &titulo, &desc);
        if let Some((texto, accion)) = boton {
            let ancho = p.medir(&texto, 12.5, Peso::Semi) + 32.0;
            let primario = accion == Accion::AbrirDescarga;
            self.boton(
                p,
                rect(x + cw - 16.0 - ancho, cy - 16.0, ancho, 32.0),
                &texto,
                None,
                primario,
                accion,
            );
        }
        y = sig;
        y += 14.0;
        y = self.cabecera(p, x, y, cw, "Archivos");
        let ruta = config::ruta().display().to_string();
        let (cy, sig) = self.fila(p, x, y, cw, "config.json", &ruta);
        self.boton(
            p,
            rect(x + cw - 16.0 - 220.0, cy - 16.0, 100.0, 32.0),
            "Abrir",
            None,
            false,
            Accion::AbrirConfig,
        );
        self.boton(
            p,
            rect(x + cw - 16.0 - 110.0, cy - 16.0, 110.0, 32.0),
            "Carpeta",
            Some(glifo::CARPETA),
            false,
            Accion::AbrirCarpeta,
        );
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Copia de la configuración",
            "Guarda tus grupos en un archivo o tráelos de otro equipo.",
        );
        self.boton(
            p,
            rect(x + cw - 16.0 - 220.0, cy - 16.0, 100.0, 32.0),
            "Exportar",
            None,
            false,
            Accion::Exportar,
        );
        self.boton(
            p,
            rect(x + cw - 16.0 - 110.0, cy - 16.0, 110.0, 32.0),
            "Importar",
            None,
            false,
            Accion::Importar,
        );
        y = sig;
        let (cy, sig) = self.fila(
            p,
            x,
            y,
            cw,
            "Salir de Orbyt",
            "Quita el ícono de la bandeja y deja de escuchar el botón.",
        );
        self.boton(
            p,
            rect(x + cw - 16.0 - 110.0, cy - 16.0, 110.0, 32.0),
            "Salir",
            Some(glifo::CERRAR),
            false,
            Accion::Salir,
        );
        y = sig + 14.0;
        y = self.cabecera(p, x, y, cw, "Cómo se usa");
        let texto = match self.cfg.modo {
            Modo::Soltar => "Mantén pulsado el botón configurado, apunta a un grupo hasta que se abra, apunta a una app y suelta. \
                             En el centro se vuelve atrás o se cancela; Escape también cancela. Un clic corto funciona como siempre. \
                             Con el teclado: números o flechas e Intro.",
            Modo::Clic => "Mantén pulsado el botón configurado hasta que salga la rueda y suéltalo. Con clic izquierdo abres un \
                           grupo y luego una app; el clic derecho o el centro vuelven atrás o cierran. Escape también cierra.",
        };
        p.parrafo(texto, rect(x, y, cw.min(640.0), 80.0), 13.0, tema::TEXTO_2);
    }
}
