# CNC GUI

A simple GCode sender for controlling CNCs.
Currently only GRBL is supported.
GCode files can be loaded and visualized before sending to the machine.

## Features

- [x] Stream GCode to GRBL microcontroller
- [x] Simple UI using Dear IMGUI
- [x] GCode visualizer with MSAA and colorscheme based on move type
- [x] Parse GRBL messages
- [x] Machine Status
- [x] GRBL GCode Validation (to slow to be practical with most GCode)
- [x] Arbitrary command sender
- [ ] Jog Controls
- [ ] Spindle and Feedrate overrides
- [ ] Work coordinate system controls
- [ ] Homing and probing cycle controls
- [ ] Configuration

## Installation

 * Install nightly Rust via [rustup.rs](https://rustup.rs/)
 * Build with the command `cargo +nightly build --release`
 * Executable will be in the `target/release/` directory