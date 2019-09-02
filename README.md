# chip8.rs

A CHIP-8 emulator library and SDL2 application written in Rust.

## Usage

chip8.rs can be used as both a standalone emulator application and
a generic CHIP-8 library.

### Application

Clone this repo, ensure that you have the [SDL2.0 development
libraries](https://github.com/Rust-SDL2/rust-sdl2#sdl20-development-libraries)
installed, and simply use `cargo run <ROM>` where `<ROM>` is the path to a
CHIP-8 ROM.

```
$ cargo run -- --help
Chip-8 Emulator 0.1
Shawn M. Chapla <shwnchpl@gmail.com>
A Rust/SDL2 CHIP-8 emulator.

USAGE:
    main <ROM>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <ROM>    Chip-8 ROM file to load.
```

### Library

As a library, chip8.rs can be used for everything from instruction decoding
to full on emulation. For an example of how to do the former, see the code
in `src/core/cpu.rs`. For an example of how to do the latter, see
`src/bin/main.rs` and all files in `src/sdl`.

## ROMs

A Google search for "chip8 roms" turns up a number of results, many/most of
which should be compatible with this emulator.

## Screenshots

### BRIX
![Screenshot of the game BRIX](/screenshots/brix.png?raw=true "BRIX")

### TETRIS
![Screenshot of the game TETRIS](/screenshots/tetris.png?raw=true "TETRIS")

## Credits

For information on the CHIP-8 architecture and how various instructions
should be implemented, I referred to the following resources:

* [The CHIP-8 Wikipedia Page](https://en.wikipedia.org/wiki/CHIP-8)
* [Cowgod's Chip-8 Techincal Reference v1.0](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM)
* [Mastering CHIP-8 by Matthew Mikolay](http://mattmik.com/files/chip8/mastering/chip8.html)

I also used [Starr Horne's](https://github.com/starrhorne)
[chip8-rust](https://github.com/starrhorne/chip8-rust) application as a
functionality reference, although I tried to avoid looking at any of his code
until my implementation was mostly complete.

## License

This project is licensed under the MIT License. See the
[LICENSE.md](LICENSE.md) file for details.
