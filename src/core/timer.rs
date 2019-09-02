use std::sync::atomic::{AtomicU8, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::driver;

type SoundDriver = Arc<Mutex<Option<Box<dyn driver::Sound>>>>;

pub struct Timer {
    pub thread: Option<thread::JoinHandle<()>>,
    pub dt: Arc<AtomicU8>,
    pub st: Arc<AtomicU8>,
    pub halt: Arc<AtomicBool>,
    pub sound_driver: SoundDriver,
}

impl Timer {
    pub fn new() -> Self {
        let dt = Arc::new(AtomicU8::new(0x00));
        let st = Arc::new(AtomicU8::new(0x00));
        let halt = Arc::new(AtomicBool::new(false));
        let sound_driver: SoundDriver = Arc::new(Mutex::new(None));

        let dt_clone = Arc::clone(&dt);
        let st_clone = Arc::clone(&st);
        let halt_clone = Arc::clone(&halt);
        let sound_driver_clone = Arc::clone(&sound_driver);

        let thread = thread::spawn(move || {
            let mut st_was_pos = false;

            loop {
                if halt_clone.load(Ordering::Relaxed) {
                    break;
                }

                let v = dt_clone.load(Ordering::Relaxed);
                if v > 0 {
                    /* Only decrement if the value didn't just change out from
                       under us. If it did, we'll catch up next cycle. Same
                       goes for the sound timer below. */
                    dt_clone.compare_and_swap(v, v - 1, Ordering::Relaxed);
                }

                let mut v = st_clone.load(Ordering::Relaxed);
                if v > 0 {
                    v = st_clone.compare_and_swap(v, v - 1, Ordering::Relaxed);
                }

                if v <= 1 && st_was_pos {
                    let mut lock = sound_driver_clone.try_lock();
                    if let Ok(ref mut mutex) = lock {
                        if let Some(sound_driver) = &mut **mutex {
                            sound_driver.stop_buzz();
                        }
                        st_was_pos = false;
                    }
                } else if  v > 1 && !st_was_pos {
                    let mut lock = sound_driver_clone.try_lock();
                    if let Ok(ref mut mutex) = lock {
                        if let Some(sound_driver) = &mut **mutex {
                            sound_driver.start_buzz();
                        }
                        st_was_pos = true;
                    }
                }

                thread::sleep(Duration::from_millis(16)); // Decent estimation of 60hz
            }
        });

        Timer {
            thread: Some(thread),
            dt,
            st,
            sound_driver,
            halt
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.halt.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
