//! Lo que hace cada clic: cambios en la configuración y en el estado de la ventana.

use super::*;

impl Ajustes {
    pub(super) fn invalidar(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub(super) fn guardar(&self) {
        if let Err(e) = config::guardar(&self.cfg) {
            eprintln!("no se pudo guardar config.json: {e}");
        }
        unsafe {
            let _ = PostMessageW(Some(self.principal), WM_ORBYT_CONFIG, WPARAM(0), LPARAM(0));
        }
    }

    pub(super) fn campo(&mut self, id: CampoId) -> &mut Campo {
        &mut self.campos[id as usize]
    }

    pub(super) fn filtrar(&mut self) {
        let q = self.campos[CampoId::Buscar as usize].texto.clone();
        self.filtradas = (0..self.apps.len())
            .filter(|&i| apps::coincide(&self.apps[i], &q))
            .collect();
        self.scroll_apps = 0.0;
    }

    /// Accesos de la lista que se está editando (la rueda principal o un grupo).
    pub(super) fn lista(&self) -> &Vec<Acceso> {
        match self.grupo.and_then(|g| self.cfg.grupos.get(g)) {
            Some(g) => &g.accesos,
            None => &self.cfg.accesos,
        }
    }

    pub(super) fn lista_mut(&mut self) -> &mut Vec<Acceso> {
        match self.grupo {
            Some(g) if g < self.cfg.grupos.len() => &mut self.cfg.grupos[g].accesos,
            _ => &mut self.cfg.accesos,
        }
    }

    pub(super) fn en_rueda(&self, destino: &str) -> bool {
        self.lista().iter().any(|a| a.destino == destino)
    }

    /// Cambia la lista que se edita.
    pub(super) fn ir_a(&mut self, grupo: Option<usize>) {
        self.grupo = grupo.filter(|&g| g < self.cfg.grupos.len());
        self.scroll_accesos = 0.0;
        self.foco = None;
        let nombre = self
            .grupo
            .map(|g| self.cfg.grupos[g].nombre.clone())
            .unwrap_or_default();
        self.campo(CampoId::NombreGrupo).poner(&nombre);
        let solo_en = self
            .grupo
            .map(|g| self.cfg.grupos[g].solo_en.join(", "))
            .unwrap_or_default();
        self.campo(CampoId::SoloEn).poner(&solo_en);
        self.elegir(None);
    }

    /// Ícono que muestra marcado el selector: el del grupo o el del editor.
    pub(super) fn icono_objetivo(&self) -> Option<String> {
        if self.selector_grupo {
            self.grupo
                .and_then(|g| self.cfg.grupos.get(g))
                .and_then(|g| g.icono.clone())
        } else {
            self.icono_editor.clone()
        }
    }

    pub(super) fn elegir(&mut self, i: Option<usize>) {
        self.seleccion = i.filter(|&i| i < self.lista().len());
        let (n, d, a) = match self.seleccion.map(|i| &self.lista()[i]) {
            Some(x) => (
                x.nombre.clone(),
                x.destino.clone(),
                x.argumentos.clone().unwrap_or_default(),
            ),
            None => Default::default(),
        };
        self.icono_editor = self.seleccion.and_then(|i| self.lista()[i].icono.clone());
        self.admin_editor = self.seleccion.is_some_and(|i| self.lista()[i].admin);
        self.campo(CampoId::Nombre).poner(&n);
        self.campo(CampoId::Destino).poner(&d);
        self.campo(CampoId::Args).poner(&a);
    }

    /// Cambia el ícono del editor; si se está editando un acceso, se guarda ya.
    pub(super) fn poner_icono(&mut self, icono: Option<String>) {
        self.selector = None;
        self.foco = None;
        if self.selector_grupo {
            if let Some(g) = self.grupo.filter(|&g| g < self.cfg.grupos.len()) {
                self.cfg.grupos[g].icono = icono;
                self.guardar();
            }
            return;
        }
        self.icono_editor = icono;
        if let Some(i) = self.seleccion {
            self.lista_mut()[i].icono = self.icono_editor.clone();
            self.guardar();
        }
    }

    pub(super) fn cargar_archivo(&mut self) {
        let ruta = self.campos[CampoId::Archivo as usize].texto.trim().to_string();
        self.archivo_cantidad = if ruta.is_empty() {
            0
        } else {
            icons::cantidad_en_archivo(&ruta).min(5000)
        };
        self.scroll_sel = 0.0;
    }

    pub(super) fn abrir_selector(&mut self) {
        let modo = match self.icono_objetivo() {
            Some(i) if i.starts_with("archivo:") => {
                if let Some((ruta, _)) = icons::partir_archivo(&i["archivo:".len()..]) {
                    self.campo(CampoId::Archivo).poner(&ruta);
                }
                ModoSel::Archivo
            }
            _ => ModoSel::Lucide,
        };
        self.cambiar_modo(modo);
    }

    pub(super) fn cambiar_modo(&mut self, modo: ModoSel) {
        self.selector = Some(modo);
        self.scroll_sel = 0.0;
        match modo {
            ModoSel::Lucide => self.foco = Some(CampoId::BuscarIcono),
            ModoSel::Archivo => {
                self.foco = Some(CampoId::Archivo);
                self.cargar_archivo();
            }
        }
    }

    /// Sin grupos no hay dónde poner un acceso: se crea uno.
    pub(super) fn asegurar_grupo(&mut self) {
        if self.grupo.is_none() {
            self.cfg.grupos.push(Grupo {
                nombre: "Principal".into(),
                ..Default::default()
            });
            self.ir_a(Some(self.cfg.grupos.len() - 1));
        }
    }

    pub(super) fn agregar_app(&mut self, i: usize) {
        self.asegurar_grupo();
        let Some(app) = self.apps.get(i) else { return };
        if let Some(ya) = self.lista().iter().position(|a| a.destino == app.destino) {
            self.elegir(Some(ya));
            return;
        }
        let nuevo = Acceso {
            nombre: app.nombre.clone(),
            destino: app.destino.clone(),
            ..Default::default()
        };
        self.lista_mut().push(nuevo);
        self.guardar();
        self.elegir(Some(self.lista().len() - 1));
        // Que el recién agregado quede a la vista.
        self.scroll_accesos = f32::MAX;
    }

    pub(super) fn guardar_formulario(&mut self) {
        let nombre = self.campos[CampoId::Nombre as usize].texto.trim().to_string();
        let destino = self.campos[CampoId::Destino as usize].texto.trim().to_string();
        let args = self.campos[CampoId::Args as usize].texto.trim().to_string();
        if nombre.is_empty() || destino.is_empty() {
            unsafe {
                let _ = MessageBeep(MB_ICONWARNING);
            }
            self.foco = Some(if nombre.is_empty() {
                CampoId::Nombre
            } else {
                CampoId::Destino
            });
            return;
        }
        let acceso = Acceso {
            nombre,
            destino,
            argumentos: (!args.is_empty()).then_some(args),
            icono: self.icono_editor.clone(),
            admin: self.admin_editor,
        };
        if self.grupo.is_none() {
            // `ir_a` limpia el editor, pero el acceso ya está leído.
            self.asegurar_grupo();
        }
        match self.seleccion {
            Some(i) => self.lista_mut()[i] = acceso,
            None => {
                self.lista_mut().push(acceso);
                self.seleccion = Some(self.lista().len() - 1);
                self.scroll_accesos = f32::MAX;
            }
        }
        self.foco = None;
        self.guardar();
    }

    pub(super) fn poner_deslizador(&mut self, d: Desliz, x: f32) {
        let Some((_, pista)) = self.pistas.iter().find(|(id, _)| *id == d) else {
            return;
        };
        let v = ((x - pista.left) / (pista.right - pista.left)).clamp(0.0, 1.0);
        match d {
            Desliz::Retardo => {
                let ms = config::RETARDO_MIN as f32 + v * (config::RETARDO_MAX - config::RETARDO_MIN) as f32;
                self.cfg.retardo_ms = ((ms / 10.0).round() * 10.0) as u32;
            }
            Desliz::Tamano => {
                let t = config::TAMANO_MIN + v * (config::TAMANO_MAX - config::TAMANO_MIN);
                self.cfg.tamano = (t * 20.0).round() / 20.0;
            }
            Desliz::Zona => {
                self.cfg.zona_central = (config::ZONA_MIN + v * (config::ZONA_MAX - config::ZONA_MIN)).round();
            }
            Desliz::EsperaGrupo => {
                let ms =
                    config::ESPERA_GRUPO_MIN as f32 + v * (config::ESPERA_GRUPO_MAX - config::ESPERA_GRUPO_MIN) as f32;
                self.cfg.espera_grupo_ms = ((ms / 50.0).round() * 50.0) as u32;
            }
            Desliz::EsperaVolver => {
                let ms = config::ESPERA_VOLVER_MIN as f32
                    + v * (config::ESPERA_VOLVER_MAX - config::ESPERA_VOLVER_MIN) as f32;
                self.cfg.espera_volver_ms = ((ms / 50.0).round() * 50.0) as u32;
            }
            Desliz::OpacidadFondo => {
                let op = config::OPACIDAD_MIN + v * (config::OPACIDAD_MAX - config::OPACIDAD_MIN);
                self.cfg.opacidad_fondo = (op * 100.0).round() / 100.0;
            }
        }
    }

    /// Ejecuta un clic. Lo que necesita un bucle de mensajes propio se devuelve para después.
    pub(super) fn ejecutar(&mut self, accion: Accion) -> Option<Diferida> {
        if accion != Accion::QuitarGrupo {
            self.confirmar_quitar = false;
        }
        if accion != Accion::CapturarAtajo {
            self.capturando = false;
        }
        match accion {
            Accion::Seccion(s) => {
                if s == Seccion::Programas {
                    self.abiertos = ventanas::programas_abiertos();
                }
                self.seccion = s;
                self.foco = None;
                self.selector = None;
            }
            Accion::AbrirEn(v) => {
                self.cfg.abrir_en = v;
                self.guardar();
            }
            Accion::Modo(m) => {
                self.cfg.modo = m;
                self.guardar();
            }
            Accion::Boton(b) => {
                self.cfg.boton = b;
                self.guardar();
            }
            Accion::InicioWindows => {
                autostart::poner(!self.inicio_windows);
                self.inicio_windows = autostart::activo();
            }
            Accion::Oscurecer => {
                self.cfg.oscurecer = !self.cfg.oscurecer;
                self.guardar();
            }
            Accion::MostrarNombres => {
                self.cfg.mostrar_nombres = !self.cfg.mostrar_nombres;
                self.guardar();
            }
            Accion::PausarPantallaCompleta => {
                self.cfg.pausar_pantalla_completa = !self.cfg.pausar_pantalla_completa;
                self.guardar();
            }
            Accion::AgregarApp(i) => self.agregar_app(i),
            Accion::Elegir(i) => self.elegir(Some(i)),
            Accion::Subir(i) if i > 0 => {
                self.lista_mut().swap(i, i - 1);
                self.guardar();
                self.elegir(Some(i - 1));
            }
            Accion::Bajar(i) if i + 1 < self.lista().len() => {
                self.lista_mut().swap(i, i + 1);
                self.guardar();
                self.elegir(Some(i + 1));
            }
            Accion::Quitar(i) if i < self.lista().len() => {
                self.lista_mut().remove(i);
                self.guardar();
                let n = self.lista().len();
                self.elegir((n > 0).then(|| i.min(n - 1)));
            }
            Accion::Contexto(g) => self.ir_a(g),
            Accion::NuevoGrupo => {
                self.cfg.grupos.push(Grupo {
                    nombre: "Nuevo grupo".into(),
                    ..Default::default()
                });
                self.guardar();
                self.ir_a(Some(self.cfg.grupos.len() - 1));
                // El nombre queda seleccionado: lo que se escriba lo reemplaza.
                self.foco = Some(CampoId::NombreGrupo);
                self.campo(CampoId::NombreGrupo).todo = true;
            }
            Accion::GrupoIzquierda => {
                if let Some(g) = self.grupo.filter(|&g| g > 0 && g < self.cfg.grupos.len()) {
                    self.cfg.grupos.swap(g, g - 1);
                    self.grupo = Some(g - 1);
                    self.guardar();
                }
            }
            Accion::GrupoDerecha => {
                if let Some(g) = self.grupo.filter(|&g| g + 1 < self.cfg.grupos.len()) {
                    self.cfg.grupos.swap(g, g + 1);
                    self.grupo = Some(g + 1);
                    self.guardar();
                }
            }
            Accion::QuitarGrupo => {
                if let Some(g) = self.grupo.filter(|&g| g < self.cfg.grupos.len()) {
                    // Un grupo puede tener muchos accesos: hace falta un segundo clic.
                    if !self.confirmar_quitar {
                        self.confirmar_quitar = true;
                    } else {
                        self.confirmar_quitar = false;
                        self.cfg.grupos.remove(g);
                        self.guardar();
                        let n = self.cfg.grupos.len();
                        self.ir_a((n > 0).then(|| g.min(n - 1)));
                    }
                }
            }
            Accion::IconoGrupo => {
                self.selector_grupo = true;
                self.abrir_selector();
            }
            Accion::EnfocarAbiertas => {
                self.cfg.enfocar_abiertas = !self.cfg.enfocar_abiertas;
                self.guardar();
            }
            Accion::Color(i) => {
                if let Some(c) = COLORES.get(i) {
                    self.cfg.color_hover = *c;
                    self.guardar();
                }
            }
            Accion::ColorOtro => return Some(Diferida::Color),
            Accion::CapturarAtajo => {
                self.capturando = !self.capturando;
                self.foco = None;
            }
            Accion::QuitarAtajo => {
                self.cfg.atajo = None;
                self.guardar();
            }
            Accion::AgregarExcluida => {
                let texto = self.campos[CampoId::Excluir as usize].texto.clone();
                self.excluir(&texto);
                self.campo(CampoId::Excluir).poner("");
            }
            Accion::QuitarExcluida(i) if i < self.cfg.excluidas.len() => {
                self.cfg.excluidas.remove(i);
                self.guardar();
            }
            Accion::ExcluirAbierto(i) => {
                if let Some(exe) = self.abiertos.get(i).cloned() {
                    self.excluir(&exe);
                }
            }
            Accion::RecargarAbiertos => self.abiertos = ventanas::programas_abiertos(),
            Accion::Admin => {
                self.admin_editor = !self.admin_editor;
                if let Some(i) = self.seleccion {
                    self.lista_mut()[i].admin = self.admin_editor;
                    self.guardar();
                }
            }
            Accion::BuscarActualizacion => {
                self.actualizacion = actualizacion::Estado::Buscando;
                actualizacion::comprobar(self.hwnd, WM_ACTUALIZACION);
            }
            Accion::AbrirDescarga => {
                if let actualizacion::Estado::Disponible { pagina, .. } = &self.actualizacion {
                    return Some(Diferida::Abrir(Acceso {
                        destino: pagina.clone(),
                        ..Default::default()
                    }));
                }
            }
            Accion::Exportar => return Some(Diferida::Exportar),
            Accion::Importar => return Some(Diferida::Importar),
            Accion::Guardar => self.guardar_formulario(),
            Accion::Nuevo => {
                self.elegir(None);
                self.foco = Some(CampoId::Nombre);
            }
            Accion::Recargar => {
                self.cargando = true;
                self.apps.clear();
                self.filtrar();
                apps::cargar_en_segundo_plano(self.hwnd, WM_APPS_LISTA, WM_APPS_ICONOS);
            }
            Accion::Examinar => return Some(Diferida::Examinar),
            Accion::ExaminarIconos => return Some(Diferida::ExaminarIconos),
            Accion::AbrirSelector => {
                self.selector_grupo = false;
                self.abrir_selector();
            }
            Accion::CerrarSelector => {
                self.selector = None;
                self.foco = None;
            }
            Accion::ModoSelector(m) => self.cambiar_modo(m),
            Accion::IconoAuto => self.poner_icono(None),
            Accion::ElegirLucide(i) => self.poner_icono(Some(format!("lucide:{}", lucide::nombre_en(i)))),
            Accion::ElegirArchivo(i) => {
                let ruta = self.campos[CampoId::Archivo as usize].texto.trim().to_string();
                self.poner_icono(Some(format!("archivo:{ruta},{i}")));
            }
            Accion::AbrirConfig => {
                return Some(Diferida::Abrir(Acceso {
                    nombre: String::new(),
                    destino: "notepad.exe".into(),
                    argumentos: Some(format!("\"{}\"", config::ruta().display())),
                    ..Default::default()
                }))
            }
            Accion::Salir => unsafe {
                // La ventana principal cierra todo (ganchos, esta ventana) al destruirse.
                let _ = PostMessageW(Some(self.principal), WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            Accion::AbrirCarpeta => {
                let dir = config::ruta()
                    .parent()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default();
                return Some(Diferida::Abrir(Acceso {
                    nombre: String::new(),
                    destino: dir,
                    ..Default::default()
                }));
            }
            _ => {}
        }
        None
    }

    /// Añade programas a los excluidos (uno o varios, separados por comas).
    pub(super) fn excluir(&mut self, texto: &str) {
        let mut cambio = false;
        for exe in config::lista_programas(texto) {
            if !self.cfg.excluidas.contains(&exe) {
                self.cfg.excluidas.push(exe);
                cambio = true;
            }
        }
        if cambio {
            self.guardar();
        }
    }

    /// Dónde quedaría lo que se arrastra si se soltara ahora.
    pub(super) fn destino_arrastre(&self, que: Que) -> Option<usize> {
        match que {
            Que::Fila(_) => {
                let n = self.lista().len();
                if n == 0 {
                    return None;
                }
                let y = self.raton.1 - (self.area_accesos.top + 4.0) + self.scroll_accesos;
                Some(((y / FILA_ACCESO).floor().max(0.0) as usize).min(n - 1))
            }
            Que::Grupo(_) => self
                .pastillas
                .iter()
                .find(|(_, r)| dentro(r, self.raton.0, self.raton.1))
                .map(|(g, _)| *g),
        }
    }

    /// Termina un arrastre: mueve el acceso o el grupo a su sitio nuevo.
    pub(super) fn soltar_arrastre(&mut self, ar: Arrastrando) {
        match ar.que {
            Que::Fila(desde) => {
                let n = self.lista().len();
                if desde < n && ar.destino < n && desde != ar.destino {
                    let acceso = self.lista_mut().remove(desde);
                    self.lista_mut().insert(ar.destino, acceso);
                    self.guardar();
                }
                self.elegir(Some(ar.destino));
            }
            Que::Grupo(desde) => {
                let n = self.cfg.grupos.len();
                if desde < n && ar.destino < n && desde != ar.destino {
                    let grupo = self.cfg.grupos.remove(desde);
                    self.cfg.grupos.insert(ar.destino, grupo);
                    self.guardar();
                }
                self.ir_a(Some(ar.destino));
            }
        }
    }

    pub(super) fn zona_en(&self, x: f32, y: f32) -> Option<Accion> {
        self.zonas.iter().rev().find(|z| dentro(&z.r, x, y)).map(|z| z.accion)
    }

    pub(super) fn zona(&mut self, r: Rect, accion: Accion) {
        self.zonas.push(Zona { r, accion });
    }

    /// Coloca el cursor del campo donde se hizo clic.
    pub(super) fn cursor_en(&mut self, id: CampoId, x: f32) {
        let Some(&(_, r, desplaz)) = self.campos_r.iter().find(|(c, _, _)| *c == id) else {
            return;
        };
        let Some(p) = self.pintor.as_mut() else { return };
        let campo = &self.campos[id as usize];
        let rel = x - (r.left + 12.0) + desplaz;
        let (mut mejor, mut dist) = (0, f32::MAX);
        let mut u16s = 0;
        for (i, c) in campo.texto.chars().chain(std::iter::once(' ')).enumerate() {
            let px = p.medir_hasta(&campo.texto, 13.0, Peso::Normal, u16s);
            if (px - rel).abs() < dist {
                dist = (px - rel).abs();
                mejor = i;
            }
            u16s += c.len_utf16();
        }
        self.campos[id as usize].ir(mejor);
    }
}
