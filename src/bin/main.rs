
use chip8::cpu::Cpu;
use chip8::driver::{Input, Sound, Display};
use sdl2::event::Event;
use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::audio::AudioDevice;
use sdl2::keyboard::Scancode;

use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;
use std::collections::HashSet;

extern crate sdl2; // TODO: Remove this if it isn't needed.
// TODO: Clean up unwraps.

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 { self.volume } else { -self.volume };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

struct SquareWaveDevice(AudioDevice<SquareWave>);

enum IOCommand {
    PollKeyboardState(u8),
    AwaitKeypress,
    RefreshDisplay(Vec<bool>),
    StartBuzz,
    StopBuzz,
    SetKeyboardChannel(Option<Sender<KeyboardState>>),
    Halt,
}

type KeyboardState = Option<u8>; // TODO: Figure out what this should actually be.

struct SdlChip8Controller {
    cido_tx: Sender<IOCommand>,                 // controller in driver out
    thread: Option<thread::JoinHandle<()>>,
}

trait Chip8UI {
    fn chip8_canvas(&self, width: u32, height: u32) ->
            Result<sdl2::render::WindowCanvas, String>;
    fn chip8_buzzer(&self) -> Result<sdl2::audio::AudioDevice<SquareWave>, String>;
}

impl Chip8UI for sdl2::Sdl {
    fn chip8_canvas(&self, width: u32, height: u32) ->
            Result<sdl2::render::WindowCanvas, String> {
        let video_subsystem = self.video().unwrap();

        let window = video_subsystem
            .window("CHIP-8 Emulator",
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

    // TODO: Clean up these long types with type aliases.
    fn chip8_buzzer(&self) -> Result<sdl2::audio::AudioDevice<SquareWave>, String> {
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

impl SdlChip8Controller {
    const SQUARE_SIZE: u32 = 20;
    const SCREEN_WIDTH: u32 = Cpu::DISPLAY_WIDTH as u32;
    const SCREEN_HEIGHT: u32 = Cpu::DISPLAY_HEIGHT as u32;

    fn new() -> Self {
        let (cido_tx, cido_rx) = channel::<IOCommand>();

        let thread = thread::spawn(move || {
            let cido_rx = cido_rx;

            let sdl_context = sdl2::init().unwrap();

            let mut canvas = sdl_context
                .chip8_canvas(Self::SCREEN_WIDTH * Self::SQUARE_SIZE,
                              Self::SCREEN_HEIGHT * Self::SQUARE_SIZE)
                .unwrap();
            let buzzer = sdl_context.chip8_buzzer().unwrap();
            let mut event_pump = sdl_context.event_pump().unwrap();
            let mut codi_tx: Option<Sender<KeyboardState>> = None;

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
                    Ok(IOCommand::StartBuzz) => buzzer.resume(),
                    Ok(IOCommand::StopBuzz) => buzzer.pause(),
                    Ok(IOCommand::SetKeyboardChannel(tx)) => codi_tx = tx,
                    Ok(IOCommand::RefreshDisplay(vram)) => {
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
                    Ok(IOCommand::PollKeyboardState(k)) => {
                        if let Some(tx) =  &codi_tx {
                            let _ = tx.send(
                                if pressed_keys.contains(&k) { Some(k) } else { None }
                            );
                        }
                    },
                    Ok(IOCommand::AwaitKeypress) => needs_key = true,
                    Ok(IOCommand::Halt) => break 'running,
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
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        });

        SdlChip8Controller {
            cido_tx,
            thread: Some(thread)
        }
    }

    fn get_sound_driver(&self) -> Box<SoundDriver> {
        Box::new(SoundDriver {
            cido_tx: self.cido_tx.clone()
        })
    }

    fn get_input_driver(&self) -> Box<InputDriver> {
        let (codi_tx, codi_rx) = channel::<KeyboardState>();
        self.cido_tx.send(IOCommand::SetKeyboardChannel(Some(codi_tx)));
        Box::new(InputDriver {
            codi_rx,
            cido_tx: self.cido_tx.clone(),
        })
    }

    fn get_display_driver(&self) -> Box<DisplayDriver> {
        Box::new(DisplayDriver {
            cido_tx: self.cido_tx.clone()
        })
    }
}

struct SoundDriver {
    cido_tx: Sender<IOCommand>,
}

struct InputDriver {
    codi_rx: Receiver<KeyboardState>,
    cido_tx: Sender<IOCommand>,
}

struct DisplayDriver {
    cido_tx: Sender<IOCommand>,
}

// TODO: Rename sound driver.
impl Sound for SoundDriver {
    fn start_buzz(&self) {
        self.cido_tx.send(IOCommand::StartBuzz).unwrap();
    }

    fn stop_buzz(&self) {
        self.cido_tx.send(IOCommand::StopBuzz).unwrap();
    }
}

// TODO: Rename input driver.
impl Input for InputDriver {
    fn poll(&self, key: u8) -> bool {
        self.cido_tx.send(IOCommand::PollKeyboardState(key)).unwrap();
        self.codi_rx.recv() == Ok(Some(key))
    }

    fn block(&self) -> u8 {
        self.cido_tx.send(IOCommand::AwaitKeypress).unwrap();

        // TODO: Is this default reasonable?
        self.codi_rx.recv().unwrap_or(Some(0)).unwrap()
    }
}

// TODO: Rename diplsay driver.
impl Display for DisplayDriver {
    fn refresh(&mut self, vram: &[bool]) {
        self.cido_tx.send(
            IOCommand::RefreshDisplay(
                vram.to_owned()
        ));
    }
}

fn main() {
    let mut cpu = Cpu::new();
    let mut ui_controller = SdlChip8Controller::new();

    cpu.set_sound_driver(Some(ui_controller.get_sound_driver()));
    cpu.set_input_driver(Some(ui_controller.get_input_driver()));
    cpu.set_display_driver(Some(ui_controller.get_display_driver()));

    // TODO: Figure out how to bring these things into sync (stop CPU w/ UI).
    // TODO: Figure out how to debounce buttons. (May not be needed).
    // TODO: Figure out how to redraw window correctly on mouse action etc.
    // TODO: Handle errors!
    // TODO: Shorten code with use statements to bring things into scope.
    // TODO: Clean up command line args etc.
    // TODO: Fix all warnings!
    let args: Vec<String> = std::env::args().collect();
    let mut f = std::fs::File::open(&args[1]).unwrap();
    let mut prog = Vec::new();
    f.read_to_end(&mut prog).unwrap();

    use std::io::prelude::*;

    cpu.load(&prog);
    loop {
        cpu.tick();//.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    // TODO: Impl drop?
    if let Some(thread) = ui_controller.thread.take() {
        thread.join();
    }
}
