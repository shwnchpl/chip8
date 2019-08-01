use std::io::Read;
use std::thread;
use std::time;

use chip8::core::cpu::Cpu;
use chip8::sdl::controller::Controller as UIController;

fn main() {
    let mut cpu = Cpu::new();
    let ui_controller = UIController::new();

    cpu.set_sound_driver(Some(ui_controller.get_sound_driver()));
    cpu.set_input_driver(Some(ui_controller.get_input_driver()));
    cpu.set_display_driver(Some(ui_controller.get_display_driver()));

    // TODO: Clean up unwraps.
    // TODO: Figure out how to bring these things into sync (stop CPU w/ UI).
    // TODO: Figure out how to redraw window correctly on mouse action etc.
    // TODO: Handle errors!
    // TODO: Clean up command line args etc.
    // TODO: Fix all warnings!
    let args: Vec<String> = std::env::args().collect();
    let mut f = std::fs::File::open(&args[1]).unwrap();
    let mut prog = Vec::new();

    f.read_to_end(&mut prog).unwrap();

    cpu.load(&prog);
    loop {
        cpu.tick();//.unwrap();
        thread::sleep(time::Duration::from_millis(2));
    }
}
