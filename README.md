# chip8.rs

A CHIP-8 emulator library and SDL2 application written in Rust.

## Usage

chip8.rs can be used as both a standalone emulator application and
a generic CHIP-8 library.

### Application

```
$ chip8 --help
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

## Roms

A Google search for "chip8 roms" turns up a number of results, many/most of
which should be compatible with this emulator.

## Screenshots

### BRIX
![Screenshot of the game BRIX](/screenshots/brix.png?raw=true "BRIX")

### TETRIS
![Screenshot of the game TETRIS](/screenshots/tetris.png?raw=true "TETRIS")

## License

This project is licensed under the MIT License. See the
[LICENSE.md](LICENSE.md) file for details.
