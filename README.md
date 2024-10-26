<p align="center">
  <!-- Really ugly workaround, but the image isn't working in crates.io without this -->
  <a href="https://github.com/Amjad50/plastic"><img alt="plastic" src="https://raw.githubusercontent.com/Amjad50/plastic/refs/heads/master/images/logo.svg" width="60%"></a>
  <p align="center">NES emulator in <em>Rust</em></p>
</p>


[![Build status](https://github.com/Amjad50/plastic/workflows/Rust/badge.svg)](https://actions-badge.atrox.dev/Amjad50/plastic/goto)
[![codecov](https://codecov.io/gh/Amjad50/plastic/branch/master/graph/badge.svg)](https://codecov.io/gh/Amjad50/plastic)
[![dependency status](https://deps.rs/repo/github/Amjad50/plastic/status.svg)](https://deps.rs/repo/github/Amjad50/plastic)
[![license](https://img.shields.io/github/license/Amjad50/plastic)](./LICENSE)
<br>
[![Crates.io Version](https://img.shields.io/crates/v/plastic_core?label=plastic_core)](https://crates.io/crates/plastic_core)
[![docs.rs](https://img.shields.io/docsrs/plastic_core)](https://docs.rs/plastic_core/latest/plastic_core/)
<br>
[![Crates.io Version](https://img.shields.io/crates/v/plastic?label=plastic)](https://crates.io/crates/plastic)
[![Crates.io Version](https://img.shields.io/crates/v/plastic_tui?label=plastic_tui)](https://crates.io/crates/plastic_tui)


**plastic** is a [NES][NES-wiki] emulator built from scratch using [Rust][Rust].

This is a personal project for fun and to experience emulating hardware and connecting them together.

- [Building and installation](#building-and-installation)
  - [Dependencies](#dependencies)
  - [Installing](#installing)
  - [Building](#building)
- [Components](#components)
- [Interfaces](#interfaces)
  - [EGui UI](#ui)
  - [TUI](#tui)
- [Controls](#controls)
  - [Keyboard](#keyboard)
  - [Gamepad](#gamepad)
- [License](#license)
- [References](#references)

### Building and installation

#### Dependencies

For linux, this project depends on `alsa` and `libudev`, you can install them using:
```sh
# ubuntu/debian
sudo apt install libasound2-dev libudev-dev
# arch
sudo pacman -S alsa-lib systemd-libs
```

#### Installing
You can install the latest version of [plastic](https://crates.io/crates/plastic) or [plastic_tui](https://crates.io/crates/plastic_tui) using cargo:
```
cargo install plastic
cargo install plastic_tui
```

Installation for Debian, Ubuntu, and Other Debian-Based Distributions

Downloading and Installing the .deb Package

To install plastic from the .deb package:

1. Download the .deb package:

 curl -L -o plastic.deb https://github.com/silverhadch/plastic-debian-build/raw/master/plastic.deb

2. Install the package using dpkg:

sudo dpkg -i plastic.deb

This will install plastic along with any required dependencies on Debian-based systems.


If you are using Arch Linux, `plastic` is available in the [official repositories](https://archlinux.org/packages/extra/x86_64/plastic/):

```
pacman -S plastic
pacman -S plastic_tui
```

#### Building
If you want to experience the latest development version, you can build `Plastic` yourself.
For example:
```
cargo run --release
```

### Components
- [x] 6502 CPU, all official and unofficial instructions with accurate timing (without BCD mode).
- [x] Picture Processing Unit, almost accurate with some small timing issues that would not effect most games.
- [x] Cartridge and INES file handling (still missing INES2.0)
- [x] Mappers:
  - [x] Mapper 0
  - [x] Mapper 1
  - [x] Mapper 2
  - [x] Mapper 3
  - [x] Mapper 4
  - [ ] Mapper 5 (Milestone)
  - [ ] Mapper 6
  - [x] Mapper 7
  - [ ] Mapper 8
  - [x] Mapper 9
  - [x] Mapper 10
  - [x] Mapper 11
  - [x] Mapper 66 
- [x] Audio Processing Unit:
  - [x] 2 Pulse wave(square)
  - [x] Triangle
  - [x] Noise
  - [x] DMC
  - [x] IRQ support
- [x] Controller:
  controllable using the keyboard and controller (tested with PS4 controller)

### Interfaces

The main emulator is at [`plastic_core`](./plastic_core/)
And its a struct `NES`, where the UI would clock it, and then
take the resulting audio and pixel buffers to handle them.

We have 2 UIs, one main and the other just for fun.

#### EGui UI
Simple ui built with [egui]

<!-- omit in toc -->
##### Advantages
1. Very simple and easy to use immediate mode UI.

#### TUI
[![TUI demo](https://img.youtube.com/vi/3wKILnY0AHU/0.jpg)](https://www.youtube.com/watch?v=3wKILnY0AHU)

This is just for fun, but it is actually working way better than
I expected. Check the [demo](https://www.youtube.com/watch?v=3wKILnY0AHU).

If you have one of these terminals mentioned [in this docs](https://docs.rs/crossterm/0.28.1/crossterm/event/struct.PushKeyboardEnhancementFlags.html)
Then you will have a much better experience, since these terminals support detecting button `Release`, normally other terminals don't have this feature, so
the input for this UI can be a bit wonky.

I used [gilrs][gilrs] for gamepad support and its working very
nicely, keyboard on the other hand is not very responsive, so it
is advised to use gamepad. Also since this uses one character for
each pixel, it is advised to use the smallest font size your
terminal emulator supports. Have fun.

### Controls
In all the UI providers I followed the same controlling scheme,
as well as the ability to reset through `<CTRL-R>`:

#### Keyboard
| keyboard | nes controller |
| -------- | -------------- |
| J | B |
| K | A |
| U | Select |
| I | Start |
| W | Up |
| S | Down |
| A | Left |
| D | Right |

#### Gamepad
| gamepad (PS4) | nes controller |
| -------- | -------------- |
| X | B |
| O | A |
| Select | Select |
| Start | Start |
| Button Up | Up |
| Button Down | Down |
| Button Left | Left |
| Button Right | Right |

For now its static, and there is no way to change it except for
doing it in the code, TODO later.

### License
This project is under [MIT](./LICENSE) license.

NES is a product and/or trademark of Nintendo Co., Ltd. Nintendo Co., Ltd. and is not affiliated in any way with Plastic or its author

### References
Most of the documentation for NES components can be found in the [NES dev wiki](http://wiki.nesdev.com/w/index.php/Nesdev_Wiki)

For the CPU(6502), [this](https://www.masswerk.at/6502/6502_instruction_set.html) has the instruction set, and I used
[Klaus2m5's tests](https://github.com/Klaus2m5/6502_65C02_functional_tests) for testing the CPU alone without the other NES components.



[NES-wiki]: https://en.wikipedia.org/wiki/Nintendo_Entertainment_System
[Rust]: https://www.rust-lang.org/
[gilrs]: https://gitlab.com/gilrs-project/gilrs
[egui]: https://github.com/emilk/egui
[ratatui]: https://github.com/ratatui/ratatui
