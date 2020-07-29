# plastic

[![Build status](https://github.com/Amjad50/plastic/workflows/Rust/badge.svg)](https://actions-badge.atrox.dev/Amjad50/plastic/goto)
[![codecov](https://codecov.io/gh/Amjad50/plastic/branch/master/graph/badge.svg)](https://codecov.io/gh/Amjad50/plastic)

**plastic** is a [NES][NES-wiki] emulator built from scratch using [Rust][Rust].

This is a personal project for fun and to experience emulating hardware and connecting them together.

### Components
- [x] 6502 CPU, all official instructions with accurate timing (without BCD mode).
- [x] Picture Processing Unit, almost accurate with some small timing issues that would not effect most games.
- [x] Cartridge and INES file handling (still missing INES2.0)
- [x] Mappers:
  - [x] Mapper 0
  - [x] Mapper 1
  - [x] Mapper 2
  - [x] Mapper 3
  - [x] Mapper 4
- [ ] Audio Processing Unit:
  - [x] 2 Pulse wave(square)
  - [x] Triangle
  - [ ] Noise
  - [ ] DMC
  - [ ] IRQ support
- [x] Controller:
  controllable using the keyboard and controller (tested with PS4 controller)

### References
Most of the documentation for NES components can be found in the [NES dev wiki](http://wiki.nesdev.com/w/index.php/Nesdev_Wiki)

For the CPU(6502), [this](https://www.masswerk.at/6502/6502_instruction_set.html) has the instruction set, and I used
[Klaus2m5's tests](https://github.com/Klaus2m5/6502_65C02_functional_tests) for testing the CPU alone without the other NES components.



[NES-wiki]: https://en.wikipedia.org/wiki/Nintendo_Entertainment_System
[Rust]: https://www.rust-lang.org/
