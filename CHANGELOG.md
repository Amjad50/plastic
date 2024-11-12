# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Debian package for `plastic`.
- `.desktop` file and `man` page for `plastic`.
- More checks in CI.
- Gamepad support for the Egui UI.

### Changed
- Updated dependancies to latest versions.

### Fixed
- Some machines having issues with initializing audio (https://github.com/Amjad50/dynwave/issues/2).
#### TUI
- Some bugs with keyboard controls on some terminals.
- Menu bar similar to the [EGui] UI to select files and save/load state, etc...
- File explorer in the terminal to open files and select games

## [0.3.1] - 2024-10-19
### Changed
- There were some issues with the README and crates.io, so fixed that, no change to the code.

## [0.3.0] - 2024-10-19
### Changed
- Major refactor to project structure, the emulation structure and UIs [#6]
- Used [EGui] instead of all the other UIs, which is cross platform
- Changed `SaveError::Other` to `SaveError::SerializationError`
- Removed unused `CartridgeError::Other` 

### Fixed
- Bugfix when Ui resumes while there is no cartridge

### Misc
- Moving to edition 2021
- Fixed module inception error in clippy
- In CI, added `rust-audit` to check for security vulnerabilities
- Improved public API, and documentation

### Testing
- Moved all blargg tests into `tests/blargg_tests.rs`
- Removed all sources from test roms files
- Added `save_load_test` to test save/load state feature

## [0.2.2] - 2020-11-07
### Added
- A new `plastic` logo. ([984590c]) and ([3c1b2fe]) 
- Icon to the GTK UI app. ([edacb0a])

### Changed
- Numerous optimizations in emulation.

### Fixed
- Windows slow performance bug, this was due to the way we handled sleep
between frames.

## [0.2.1] - 2020-09-09
### Fixed
- Windows GUI crash bug. ([0626860])
- Labels in Save/Load state menus not updating when running a new game. ([853705a])
- Backend exit (UI still present), in the case of loading a new game that has an unsupported mapper. ([3972696])
- `CHANGELOG` compare releases links.

## [0.2.0] - 2020-09-09
### Added
- Cartridge with [INES2.0] header format support.
- Mapper 10 ([MMC4]).
- Mapper 11 ([Color Dreams]).
- [Mapper 12].
- Mapper 66 ([GxROM]).
- PPU color emphasis support. ([69c70be])
- Native windows GUI provider.
- Save state feature.
- This `CHANGELOG` file.

### Changed
- Removed all `Rc<RefCell<>>` usages from the APU structure.

### Removed
- APU [Filter] which was taken from the [pinky] project.

### Fixed
- Mappers bank out of range bug. ([e42f3ee] & [8b2ba0e])
- Most PPU timing accuracy (Now most tests pass!).
- Some APU timing and internal accuracy.
- GTK application was able to run only a single instance at a time. ([d29c27d])
- Slow emulation in windows due to Windows's Sleep API. ([1ac20ac])

## [0.1.0] - 2020-08-20
### Added
- Initial implementation of the CPU6502.
- [Cirrus] CI system.
- Interrupt support for the CPU.
- Initial implementation of the PPU2C02.
- Initial implementation of the cartridge.
- Mapper 0 ([NROM]).
- TV (display).
- CHR RAM support in cartridge.
- [SFML] GUI.
- PPU Sprite rendering support.
- Mapper 1 ([MMC1]).
- Mapper 2 ([UxROM]).
- [Mapper 3].
- Mapper 4 ([MMC3]).
- [Emulator tests] for CPU and PPU.
- Added joystick (PS4 controller) support for [SFML] GUI.
- Initial implementation of the APU using sine waves (it was so bad).
- [MIT] licence.
- [Codecov] coverage meter.
- Completed implementation of IRQ support for Mapper 4 ([MMC3]).
- Support for SRAM (battery powered RAM) to save into `*.nes.sav` file.
- Implementation of CPU unofficial instructions.
- Mapper 7 ([AxROM]).
- Mapper 9 ([MMC2]).
- [GTK] GUI.
- TUI UI (because its fun).
- joystick support fo TUI through [Gilrs] to make it easier to play.
- IRQ support for APU.
- Ability to **Pause**, **Resume** and **Reset** emulation.
- Support for file drag in [GTK] GUI.

### Changed
- Moved from [Cirrus] to [GithubActions] for CI.
- Major rewrite for APU channels and internals (the right way to do it).

### Fixed
- This is the first release and has **SO** many rewrites and bug fixes.

[Unreleased]: https://github.com/Amjad50/plastic/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/Amjad50/plastic/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Amjad50/plastic/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/Amjad50/plastic/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/Amjad50/plastic/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Amjad50/plastic/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Amjad50/plastic/releases/tag/v0.1.0

[69c70be]: https://github.com/Amjad50/plastic/commit/69c70be
[e42f3ee]: https://github.com/Amjad50/plastic/commit/e42f3ee
[8b2ba0e]: https://github.com/Amjad50/plastic/commit/8b2ba0e
[d29c27d]: https://github.com/Amjad50/plastic/commit/d29c27d
[1ac20ac]: https://github.com/Amjad50/plastic/commit/1ac20ac
[0626860]: https://github.com/Amjad50/plastic/commit/0626860
[853705a]: https://github.com/Amjad50/plastic/commit/853705a
[3972696]: https://github.com/Amjad50/plastic/commit/3972696
[984590c]: https://github.com/Amjad50/plastic/commit/984590c
[3c1b2fe]: https://github.com/Amjad50/plastic/commit/3c1b2fe
[edacb0a]: https://github.com/Amjad50/plastic/commit/edacb0a

[Emulator tests]: http://wiki.nesdev.com/w/index.php/Emulator_tests

[INES2.0]: https://wiki.nesdev.com/w/index.php/NES_2.0

[NROM]: https://wiki.nesdev.com/w/index.php/INES_Mapper_000
[MMC1]: https://wiki.nesdev.com/w/index.php/INES_Mapper_001
[UxROM]: https://wiki.nesdev.com/w/index.php/INES_Mapper_002
[Mapper 3]: https://wiki.nesdev.com/w/index.php/INES_Mapper_003
[MMC3]: https://wiki.nesdev.com/w/index.php/INES_Mapper_004
[AxROM]: https://wiki.nesdev.com/w/index.php/INES_Mapper_007
[MMC2]: https://wiki.nesdev.com/w/index.php/INES_Mapper_009
[MMC4]: https://wiki.nesdev.com/w/index.php/INES_Mapper_010
[Color Dreams]: https://wiki.nesdev.com/w/index.php/INES_Mapper_011
[Mapper 12]: https://wiki.nesdev.com/w/index.php/INES_Mapper_012
[GxROM]: https://wiki.nesdev.com/w/index.php/INES_Mapper_066

[Filter]: https://github.com/koute/pinky/blob/17c51a1e96a6eead0b340031bc97634e7261b928/nes/src/filter.rs
[pinky]: https://github.com/koute/pinky

[MIT]: https://github.com/Amjad50/plastic/blob/0ca36f10174829647469f8980b7e3fc282e7151a/LICENSE

[Cirrus]: https://cirrus-ci.org/
[GithubActions]: https://github.com/features/actions

[SFML]: https://www.sfml-dev.org/
[GTK]: https://www.gtk.org/
[Gilrs]: https://gitlab.com/gilrs-project/gilrs
[EGui]: https://github.com/emilk/egui
