//! Teclado y portapapeles de los campos de texto.

use super::*;

fn portapapeles() -> Option<String> {
    unsafe {
        OpenClipboard(None).ok()?;
        let texto = (|| {
            let h = GetClipboardData(13 /* CF_UNICODETEXT */).ok()?;
            let global = windows::Win32::Foundation::HGLOBAL(h.0);
            let p = GlobalLock(global) as *const u16;
            if p.is_null() {
                return None;
            }
            let mut n = 0;
            while *p.add(n) != 0 {
                n += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
            let _ = GlobalUnlock(global);
            Some(s)
        })();
        let _ = CloseClipboard();
        texto
    }
}

impl Ajustes {
    pub(super) fn caracter(&mut self, c: u16) -> Option<Diferida> {
        if c == 0x1B && self.selector.is_some() {
            self.selector = None;
            self.foco = None;
            return None;
        }
        let Some(id) = self.foco else {
            if c == 0x1B {
                unsafe {
                    let _ = DestroyWindow(self.hwnd);
                }
            }
            return None;
        };
        self.cursor_visible = true;
        match c {
            0x08 => self.campo(id).borrar_atras(),
            0x01 => self.campo(id).todo = true,
            0x16 => {
                if let Some(t) = portapapeles() {
                    self.campo(id).insertar(&t);
                }
            }
            0x0D => match id {
                CampoId::Buscar => {
                    if let Some(&i) = self.filtradas.first() {
                        self.agregar_app(i);
                    }
                }
                CampoId::BuscarIcono => {
                    if let Some(&i) = self.lucide_filtrados.first() {
                        self.poner_icono(Some(format!("lucide:{}", lucide::nombre_en(i))));
                    }
                }
                CampoId::Archivo => self.cargar_archivo(),
                CampoId::NombreGrupo | CampoId::SoloEn => self.foco = None,
                CampoId::Excluir => {
                    let texto = self.campos[id as usize].texto.clone();
                    self.excluir(&texto);
                    self.campo(id).poner("");
                }
                _ => self.guardar_formulario(),
            },
            0x1B => self.foco = None,
            0x09 if self.selector.is_some() => {}
            0x09 => {
                let orden = [CampoId::Buscar, CampoId::Nombre, CampoId::Destino, CampoId::Args];
                let i = orden.iter().position(|x| *x == id).unwrap_or(0);
                self.foco = Some(orden[(i + 1) % orden.len()]);
            }
            0xD800..=0xDBFF => self.sustituto = Some(c),
            0xDC00..=0xDFFF => {
                if let Some(alto) = self.sustituto.take() {
                    let s = String::from_utf16_lossy(&[alto, c]);
                    self.campo(id).insertar(&s);
                }
            }
            c if c >= 0x20 && c != 0x7F => {
                let s = String::from_utf16_lossy(&[c]);
                self.campo(id).insertar(&s);
            }
            _ => {}
        }
        if c != 0x0D && c != 0x09 && c != 0x1B {
            self.al_cambiar(id);
        }
        None
    }

    /// Lo que depende del texto de un campo (filtros de búsqueda, archivo de íconos).
    pub(super) fn al_cambiar(&mut self, id: CampoId) {
        match id {
            CampoId::Buscar => self.filtrar(),
            CampoId::BuscarIcono => {
                self.lucide_filtrados = lucide::buscar(&self.campos[id as usize].texto);
                self.scroll_sel = 0.0;
            }
            CampoId::Archivo => self.cargar_archivo(),
            CampoId::SoloEn => {
                // Los programas del grupo se guardan mientras se escriben.
                let lista = config::lista_programas(&self.campos[id as usize].texto);
                if let Some(g) = self.grupo.filter(|&g| g < self.cfg.grupos.len()) {
                    self.cfg.grupos[g].solo_en = lista;
                    self.guardar();
                }
            }
            CampoId::NombreGrupo => {
                // El nombre del grupo se guarda mientras se escribe.
                let nombre = self.campos[id as usize].texto.trim().to_string();
                if let Some(g) = self.grupo.filter(|&g| g < self.cfg.grupos.len()) {
                    self.cfg.grupos[g].nombre = nombre;
                    self.guardar();
                }
            }
            _ => {}
        }
    }

    /// Una tecla mientras se espera el atajo: con una combinación válida se guarda; Escape
    /// cancela y Retroceso o Supr lo quitan. Un modificador solo no hace nada (se sigue esperando).
    pub(super) fn capturar(&mut self, vk: u32) {
        let abajo = |t: VIRTUAL_KEY| unsafe { GetKeyState(t.0 as i32) } < 0;
        let mut mods = 0;
        for (teclas, m) in [
            (&[VK_CONTROL][..], atajo::CTRL),
            (&[VK_MENU][..], atajo::ALT),
            (&[VK_SHIFT][..], atajo::MAYUS),
            (&[VK_LWIN, VK_RWIN][..], atajo::WIN),
        ] {
            if teclas.iter().any(|t| abajo(*t)) {
                mods |= m;
            }
        }
        let es = |t: VIRTUAL_KEY| t.0 as u32 == vk;
        if mods == 0 && es(VK_ESCAPE) {
            self.capturando = false;
        } else if mods == 0 && (es(VK_BACK) || es(VK_DELETE)) {
            self.cfg.atajo = None;
            self.capturando = false;
            self.guardar();
        } else if mods == atajo::ALT && es(VK_F4) {
            // Alt+F4 cierra ventanas: no se puede quedar como atajo.
        } else if let Some(texto) = atajo::escribir(mods, vk) {
            self.cfg.atajo = Some(texto);
            self.capturando = false;
            self.guardar();
        }
    }

    pub(super) fn tecla(&mut self, vk: u16) -> bool {
        let Some(id) = self.foco else { return false };
        self.cursor_visible = true;
        let campo = self.campo(id);
        match VIRTUAL_KEY(vk) {
            VK_LEFT => campo.mover(-1),
            VK_RIGHT => campo.mover(1),
            VK_HOME => campo.ir(0),
            VK_END => campo.ir(usize::MAX),
            VK_DELETE => {
                campo.borrar_delante();
                self.al_cambiar(id);
            }
            _ => return false,
        }
        true
    }
}
