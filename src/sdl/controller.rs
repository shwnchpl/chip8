use std::collections::HashSet;
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time;

use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::render::WindowCanvas;

use crate::core::cpu::Cpu;

use super::io;
use super::io::{Buzzer, SquareWave};
use super::driver::{InputDriver, SoundDriver, DisplayDriver};

type Result<T> = std::result::Result<T, String>;

trait Chip8UI {
    fn chip8_canvas(&self, title: &str, width: u32, height: u32)
            -> Result<WindowCanvas>;
    fn chip8_buzzer(&self) -> Result<Buzzer>;
}

impl Chip8UI for sdl2::Sdl {
    fn chip8_canvas(&self, title: &str, width: u32, height: u32)
            -> Result<WindowCanvas> {
        let video_subsystem = self.video()?;

        let window = video_subsystem
            .window(title,
                    width,
                    height)
            .position_centered()
            .build()
            .map_err(|err| err.to_string() )?;

        window
            .into_canvas()
            .target_texture()
            .present_vsync()
            .build()
            .map_err(|err| err.to_string() )
    }

    fn chip8_buzzer(&self) -> Result<Buzzer> {
        let audio_subsystem = self.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(1),
            samples: None
        };

        audio_subsystem
            .open_playback(None, &desired_spec, |spec| {
                SquareWave {
                    phase_inc: 440.0 / spec.freq as f32,
                    phase: 0.0,
                    volume: 0.25,
                }
            })
    }
}
// cido - controller in driver out
// codi - controller out driver in
pub struct Controller {
    cido_tx: Sender<io::Command>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for Controller {
    fn drop(&mut self) {
        // TODO: Send the halt message?
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

impl Controller {
    const SQUARE_SIZE: u32 = 20;
    const SCREEN_WIDTH: u32 = Cpu::DISPLAY_WIDTH as u32;
    const SCREEN_HEIGHT: u32 = Cpu::DISPLAY_HEIGHT as u32;
    const WINDOW_TITLE: &'static str = "CHIP-8 Emulator";

    pub fn new() -> Self {
        let (cido_tx, cido_rx) = channel::<io::Command>();

        let thread = thread::spawn(move || {
            let cido_rx = cido_rx;

            let sdl_context = sdl2::init().unwrap();

            let mut canvas = sdl_context
                .chip8_canvas(
                    Self::WINDOW_TITLE,
                    Self::SCREEN_WIDTH * Self::SQUARE_SIZE,
                    Self::SCREEN_HEIGHT * Self::SQUARE_SIZE)
                .unwrap();
            let buzzer = sdl_context.chip8_buzzer().unwrap();
            let mut event_pump = sdl_context.event_pump().unwrap();
            let mut codi_tx: Option<Sender<io::Key>> = None;

            // canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
            canvas.clear();
            canvas.present();

            let mut needs_key = false;

            'running: loop {
                // TODO: Consider mpsc select?
                let pressed_keys: HashSet<u8> = event_pump
                    .keyboard_state()
                    .pressed_scancodes()
                    .filter_map(|s| match s {
                        Scancode::Num1 => Some(0x1),
                        Scancode::Num2 => Some(0x2),
                        Scancode::Num3 => Some(0x3),
                        Scancode::Num4 => Some(0xc),
                        Scancode::Q => Some(0x4),
                        Scancode::W => Some(0x5),
                        Scancode::E => Some(0x6),
                        Scancode::R => Some(0xd),
                        Scancode::A => Some(0x7),
                        Scancode::S => Some(0x8),
                        Scancode::D => Some(0x9),
                        Scancode::F => Some(0xe),
                        Scancode::Z => Some(0xa),
                        Scancode::X => Some(0x0),
                        Scancode::C => Some(0xb),
                        Scancode::V => Some(0xf),
                        _ => None,
                    })
                    .collect();

                match cido_rx.try_recv() {
                    Ok(io::Command::BuzzStart) => buzzer.resume(),
                    Ok(io::Command::BuzzStop) => buzzer.pause(),
                    Ok(io::Command::DisplayRefresh(vram)) => {
                        let light = sdl2::pixels::Color::RGB(255, 255, 255);
                        let dark = sdl2::pixels::Color::RGB(0, 0, 0);
                        for (i, px_set) in vram.iter().enumerate() {
                            canvas.set_draw_color(
                                if *px_set { light } else { dark
                                }
                            );
                            let i = i as u32;
                            let x = (i % Self::SCREEN_WIDTH) * Self::SQUARE_SIZE;
                            let y = (i / Self::SCREEN_WIDTH) * Self::SQUARE_SIZE;
                            canvas.fill_rect(
                                sdl2::rect::Rect::new(
                                    x as i32, y as i32, Self::SQUARE_SIZE, Self::SQUARE_SIZE
                                )
                            );
                        }
                        canvas.present();
                    },
                    Ok(io::Command::KeyBlock) => needs_key = true,
                    Ok(io::Command::KeyChanSet(tx)) => codi_tx = tx,
                    Ok(io::Command::KeyPoll(k)) => {
                        if let Some(tx) =  &codi_tx {
                            let _ = tx.send(
                                if pressed_keys.contains(&k) { Some(k) } else { None }
                            );
                        }
                    },
                    Ok(io::Command::Quit) => break 'running,
                    _ => (),
                }

                if needs_key && !pressed_keys.is_empty() {
                    if let Some(tx) = &codi_tx {
                        tx.send(Some(*pressed_keys.iter().nth(0).unwrap()));
                    }
                    needs_key = false;
                }

                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit {..} => {
                            break 'running;
                        },
                        _ => ()
                    }
                }

                // TODO: Does this make sense?
                thread::sleep(time::Duration::from_millis(2));
            }
        });

        Controller {
            cido_tx,
            thread: Some(thread)
        }
    }

    pub fn get_sound_driver(&self) -> Box<SoundDriver> {
        Box::new(SoundDriver {
            cido_tx: self.cido_tx.clone()
        })
    }

    pub fn get_input_driver(&self) -> Box<InputDriver> {
        let (codi_tx, codi_rx) = channel::<io::Key>();
        self.cido_tx.send(io::Command::KeyChanSet(Some(codi_tx)));
        Box::new(InputDriver {
            codi_rx,
            cido_tx: self.cido_tx.clone(),
        })
    }

    pub fn get_display_driver(&self) -> Box<DisplayDriver> {
        Box::new(DisplayDriver {
            cido_tx: self.cido_tx.clone()
        })
    }
}
