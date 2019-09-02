use std::fs::File;
use std::io;
use std::io::{Error, ErrorKind};
use std::io::prelude::*;
use std::thread;
use std::time;

use clap::{Arg, App};

use chip8::core::cpu::Cpu;
use chip8::sdl::controller::Controller as UIController;

fn main() -> io::Result<()> {
    let matches = App::new("Chip-8 Emulator")
        .version("0.1")
        .author("Shawn M. Chapla <shwnchpl@gmail.com>")
        .about("A Rust/SDL2 CHIP-8 emulator.")
        .arg(Arg::with_name("ROM")
             .help("Chip-8 ROM file to load.")
             .required(true)
             .index(1))
        .get_matches();

    let rom_path = matches.value_of("ROM").unwrap();
    let mut f = File::open(rom_path)?;
    let mut prog = Vec::new();

    f.read_to_end(&mut prog)?;

    let ui_controller = UIController::new();
    let mut cpu = Cpu::new();

    cpu.set_sound_driver(Some(ui_controller.get_sound_driver()));
    cpu.set_input_driver(Some(ui_controller.get_input_driver()));
    cpu.set_display_driver(Some(ui_controller.get_display_driver()));

    cpu.load(&prog)
        .map_err(
            |e| Error::new(ErrorKind::InvalidData, e.to_string())
        )?;

    while ui_controller.alive() {
        if let Err(e) = cpu.tick() {
            if e.fatal() {
                println!("fatal CPU error: {:?}", e);
                break;
            }
        }
        thread::sleep(time::Duration::from_millis(2));
    }

    Ok(())
}
