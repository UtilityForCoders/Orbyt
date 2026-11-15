//! Pulso de fotogramas limitado a unos 60 Hz.
//!
//! Mientras la rueda está abierta, un hilo aparte avisa a la ventana. Limitar el pulso evita que
//! monitores de 144 o 240 Hz multipliquen el trabajo sin aportar una diferencia apreciable en
//! una animación tan corta. La espera no puede hacerse en el hilo principal: ahí viven los
//! ganchos de entrada, que tienen que responder enseguida.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::OnceLock;
use std::thread::{self, Thread};
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Toca pintar un fotograma.
pub const WM_ORBYT_FOTOGRAMA: u32 = WM_APP + 6;

static ACTIVO: AtomicBool = AtomicBool::new(false);
/// Hay un aviso en la cola sin atender: no se manda otro hasta que se atienda, para que un hilo
/// principal ocupado no acumule fotogramas atrasados.
static EN_COLA: AtomicBool = AtomicBool::new(false);
static VENTANA: AtomicIsize = AtomicIsize::new(0);
static HILO: OnceLock<Thread> = OnceLock::new();

/// Empieza a mandar `WM_ORBYT_FOTOGRAMA` a `hwnd` hasta unas 60 veces por segundo. Si ya estaba
/// en marcha no hace nada: despertar al hilo a destiempo adelantaría un fotograma.
pub fn empezar(hwnd: HWND) {
    VENTANA.store(hwnd.0 as isize, Ordering::SeqCst);
    if ACTIVO.load(Ordering::SeqCst) {
        return;
    }
    EN_COLA.store(false, Ordering::SeqCst);
    ACTIVO.store(true, Ordering::SeqCst);
    // Un único hilo para toda la vida del programa: se duerme cuando la rueda se cierra.
    HILO.get_or_init(|| thread::Builder::new().name("ritmo".into()).spawn(bucle).expect("hilo de fotogramas").thread().clone())
        .unpark();
}

pub fn parar() {
    ACTIVO.store(false, Ordering::SeqCst);
}

/// Lo llama el gancho del ratón con la rueda abierta. Con el pulso parado (nada que animar), un
/// movimiento pide un fotograma suelto; así la rueda quieta no despierta a la CPU 60 veces por
/// segundo. Con el pulso en marcha no hace falta: ya hay fotogramas.
pub fn al_mover() {
    if ACTIVO.load(Ordering::Relaxed) || EN_COLA.swap(true, Ordering::SeqCst) {
        return;
    }
    let hwnd = HWND(VENTANA.load(Ordering::Relaxed) as *mut _);
    if unsafe { PostMessageW(Some(hwnd), WM_ORBYT_FOTOGRAMA, WPARAM(0), LPARAM(0)) }.is_err() {
        EN_COLA.store(false, Ordering::SeqCst);
    }
}

/// Llamar al atender `WM_ORBYT_FOTOGRAMA`, antes de pintar.
pub fn atendido() {
    EN_COLA.store(false, Ordering::SeqCst);
}

fn bucle() {
    loop {
        while !ACTIVO.load(Ordering::SeqCst) {
            thread::park();
        }
        thread::park_timeout(Duration::from_millis(16));
        if ACTIVO.load(Ordering::SeqCst) && !EN_COLA.swap(true, Ordering::SeqCst) {
            let hwnd = HWND(VENTANA.load(Ordering::SeqCst) as *mut _);
            if unsafe { PostMessageW(Some(hwnd), WM_ORBYT_FOTOGRAMA, WPARAM(0), LPARAM(0)) }.is_err() {
                EN_COLA.store(false, Ordering::SeqCst);
            }
        }
    }
}
