use super::{
    error::{CartridgeError, SramError},
    mapper::{Mapper, MappingResult},
    mappers::*,
};
use common::{interconnection::CpuIrqProvider, Bus, Device, MirroringMode, MirroringProvider};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub struct Cartridge {
    file_path: Box<Path>,
    // header
    _size_prg: u8,
    _size_chr: u8,
    _is_chr_ram: bool,
    _mapper_id: u8,
    mirroring_vertical: bool,
    contain_sram: bool,
    sram_size: u8,
    _contain_trainer: bool,
    use_4_screen_mirroring: bool,
    _vs_unisystem: bool,       // don't know what is this (flag 7)
    _playchoice_10_hint: bool, // not used
    _is_nes_2: bool,

    _trainer_data: Vec<u8>,
    pub(crate) prg_data: Vec<u8>,
    pub(crate) chr_data: Vec<u8>,
    sram_data: Vec<u8>,

    mapper: Box<dyn Mapper>,

    is_empty: bool,
}

impl Cartridge {
    // TODO: not sure if it should consume the file or not
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, CartridgeError> {
        if let Some(extension) = file_path.as_ref().extension() {
            if extension == "nes" {
                let mut file = File::open(file_path.as_ref())?;

                let mut header = [0; 16];
                file.read_exact(&mut header)?;

                // decode header
                Cartridge::check_magic(&header[0..4])?;

                let size_prg = header[4];
                let is_chr_ram = header[5] == 0;
                let size_chr = if is_chr_ram { 1 } else { header[5] };

                let mirroring_vertical = header[6] & 1 != 0;
                header[6] >>= 1;
                let contain_sram = header[6] & 1 != 0;
                header[6] >>= 1;
                let contain_trainer = header[6] & 1 != 0;
                header[6] >>= 1;
                let use_4_screen_mirroring = header[6] & 1 != 0;
                header[6] >>= 1;
                let lower_mapper = header[6]; // the rest

                let vs_unisystem = header[7] & 1 != 0;
                header[7] >>= 1;
                let _playchoice_10_hint = header[7] & 1 != 0;
                header[7] >>= 1;
                let is_nes_2 = (header[7] & 0b11) == 2;
                header[7] >>= 2;
                let upper_mapper = header[7]; // the rest

                // in 8kb units
                let sram_size = if header[8] == 0 { 1 } else { header[8] };

                let sram_data = if contain_sram {
                    // try to load old save data
                    if let Ok(data) =
                        Self::load_sram_file(file_path.as_ref(), sram_size as usize * 1024 * 8)
                    {
                        data
                    } else {
                        vec![0; sram_size as usize * 1024 * 8]
                    }
                } else {
                    vec![0; sram_size as usize * 1024 * 8]
                };

                let mapper_id = upper_mapper << 4 | lower_mapper;

                // initialize the mapper first, so that if it is not supported yet,
                // panic
                let mapper = Self::get_mapper(mapper_id, size_prg, size_chr, is_chr_ram, sram_size)
                    .ok_or(CartridgeError::MapperNotImplemented(mapper_id))?;

                let mut trainer_data = Vec::new();

                // read training data if present
                if contain_trainer {
                    trainer_data.resize(512, 0);
                    file.read_exact(&mut trainer_data)?;
                }

                // read PRG data
                let mut prg_data = vec![0; (size_prg as usize) * 16 * 1024];
                file.read_exact(&mut prg_data)?;

                // read CHR data
                let mut chr_data = vec![0; (size_chr as usize) * 8 * 1024];
                if !is_chr_ram {
                    file.read_exact(&mut chr_data)?;
                }

                if is_nes_2 {
                    // print a warning message just to know which games need INES2.
                    eprintln!(
                        "[WARN], the cartridge header is in INES2.0 format, but \
                this emulator only supports INES1.0, the game might work \
                but mostly it will be buggy"
                    );
                }

                // there are missing parts
                let current = file.seek(SeekFrom::Current(0))?;
                let end = file.seek(SeekFrom::End(0))?;
                if current != end {
                    Err(CartridgeError::TooLargeFile(end - current))
                } else {
                    Ok(Self {
                        file_path: file_path.as_ref().to_path_buf().into_boxed_path(),
                        _size_prg: size_prg,
                        _size_chr: size_chr,
                        _is_chr_ram: is_chr_ram,
                        _mapper_id: mapper_id,
                        mirroring_vertical,
                        contain_sram,
                        sram_size,
                        _contain_trainer: contain_trainer,
                        use_4_screen_mirroring,
                        _vs_unisystem: vs_unisystem,
                        _playchoice_10_hint,
                        _is_nes_2: is_nes_2,
                        _trainer_data: trainer_data,
                        prg_data,
                        chr_data,
                        sram_data,
                        mapper,

                        is_empty: false,
                    })
                }
            } else {
                Err(CartridgeError::ExtensionError)
            }
        } else {
            Err(CartridgeError::ExtensionError)
        }
    }

    pub fn new_without_file() -> Self {
        Self {
            // should not be used
            file_path: Path::new("").to_path_buf().into_boxed_path(),
            _size_prg: 0,
            _size_chr: 0,
            _is_chr_ram: false,
            _mapper_id: 0,
            mirroring_vertical: false,
            contain_sram: false,
            sram_size: 0,
            _contain_trainer: false,
            use_4_screen_mirroring: false,
            _vs_unisystem: false,
            _playchoice_10_hint: false,
            _is_nes_2: false,
            _trainer_data: Vec::new(),
            prg_data: Vec::new(),
            chr_data: Vec::new(),
            sram_data: Vec::new(),
            mapper: Box::new(Mapper0::new()),

            is_empty: true,
        }
    }

    pub fn is_vertical_mirroring(&self) -> bool {
        self.mirroring_vertical
    }

    fn check_magic(header: &[u8]) -> Result<(), CartridgeError> {
        let real = [0x4E, 0x45, 0x53, 0x1A];

        if header == real {
            Ok(())
        } else {
            Err(CartridgeError::HeaderError)
        }
    }

    fn get_mapper(
        mapper_id: u8,
        prg_count: u8,
        chr_count: u8,
        is_chr_ram: bool,
        sram_size: u8,
    ) -> Option<Box<dyn Mapper>> {
        let mut mapper: Box<dyn Mapper> = match mapper_id {
            0 => Box::new(Mapper0::new()),
            1 => Box::new(Mapper1::new()),
            2 => Box::new(Mapper2::new()),
            3 => Box::new(Mapper3::new()),
            4 => Box::new(Mapper4::new()),
            7 => Box::new(Mapper7::new()),
            9 => Box::new(Mapper9::new()),
            10 => Box::new(Mapper10::new()),
            11 => Box::new(Mapper11::new()),
            _ => {
                return None;
            }
        };

        // should always call init in a new mapper, as it is the only way
        // they share a constructor
        mapper.init(prg_count, is_chr_ram, chr_count, sram_size);

        Some(mapper)
    }

    fn load_sram_file<P: AsRef<Path>>(path: P, sram_size: usize) -> Result<Vec<u8>, SramError> {
        let path = path.as_ref().with_extension("nes.sav");
        println!("Loading SRAM file data from {:?}", path);

        let mut file = File::open(path)?;
        let mut result = vec![0; sram_size];

        file.read_exact(&mut result)
            .map_err(|_| SramError::SramFileSizeDoesNotMatch)?;

        Ok(result)
    }

    fn save_sram_file(&self) -> Result<(), SramError> {
        let path = self.file_path.with_extension("nes.sav");
        println!("Writing SRAM file data to {:?}", path);

        let mut file = File::create(&path)?;

        let size = file.write(&self.sram_data)?;

        if size != self.sram_size as usize * 1024 * 8 {
            file.sync_all()?;
            // remove the file so it will not be loaded next time the game is run
            std::fs::remove_file(path).expect("Could not remove `nes.sav` file");
            Err(SramError::FailedToSaveSramFile)
        } else {
            Ok(())
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_empty
    }
}

impl Bus for Cartridge {
    fn read(&self, address: u16, device: Device) -> u8 {
        if self.is_empty {
            return match device {
                Device::CPU => 0xEA, // NOP instruction just in case, this
                Device::PPU => 0x00, // should not be called
            };
        }

        let result = self.mapper.map_read(address, device);

        if let MappingResult::Allowed(new_address) = result {
            match device {
                Device::CPU => match address {
                    0x6000..=0x7FFF => {
                        *self.sram_data.get(new_address).expect("SRAM out of bounds")
                    }
                    0x8000..=0xFFFF => *self.prg_data.get(new_address).expect("PRG out of bounds"),
                    _ => {
                        unreachable!();
                    }
                },
                Device::PPU => {
                    if address <= 0x1FFF {
                        *self.chr_data.get(new_address).expect("CHR out of bounds")
                    } else {
                        unreachable!();
                    }
                }
            }
        } else {
            0
        }
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        if self.is_empty {
            return;
        }

        // send the write signal, this might trigger bank change
        let result = self.mapper.map_write(address, data, device);

        if let MappingResult::Allowed(new_address) = result {
            match device {
                Device::CPU => match address {
                    0x6000..=0x7FFF => {
                        *self
                            .sram_data
                            .get_mut(new_address)
                            .expect("SRAM out of bounds") = data;
                    }
                    0x8000..=0xFFFF => {
                        *self
                            .prg_data
                            .get_mut(new_address)
                            .expect("PRG out of bounds") = data;
                    }
                    _ => {
                        unreachable!();
                    }
                },
                Device::PPU => {
                    if address <= 0x1FFF {
                        *self
                            .chr_data
                            .get_mut(new_address)
                            .expect("CHR out of bounds") = data;
                    } else {
                        unreachable!();
                    }
                }
            }
        }
    }
}

impl MirroringProvider for Cartridge {
    fn mirroring_mode(&self) -> MirroringMode {
        if self.is_empty {
            //anything
            return MirroringMode::Vertical;
        }

        if self.use_4_screen_mirroring {
            MirroringMode::FourScreen
        } else {
            if self.mapper.is_hardwired_mirrored() {
                if self.mirroring_vertical {
                    MirroringMode::Vertical
                } else {
                    MirroringMode::Horizontal
                }
            } else {
                self.mapper.nametable_mirroring()
            }
        }
    }
}

impl Drop for Cartridge {
    fn drop(&mut self) {
        if !self.is_empty && self.contain_sram {
            self.save_sram_file().unwrap();
        }
    }
}

impl CpuIrqProvider for Cartridge {
    fn is_irq_change_requested(&self) -> bool {
        if self.is_empty {
            return false;
        }

        self.mapper.is_irq_pin_state_changed_requested()
    }

    fn irq_pin_state(&self) -> bool {
        self.mapper.irq_pin_state()
    }

    fn clear_irq_request_pin(&mut self) {
        self.mapper.clear_irq_request_pin();
    }
}
