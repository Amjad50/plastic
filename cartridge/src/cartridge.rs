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

struct INesHeader {
    // in 16kb units
    prg_rom_size: u16,
    // in 8kb units
    chr_rom_size: u16,
    is_chr_ram: bool,
    hardwired_mirroring_vertical: bool,
    has_prg_ram_battery: bool,
    contain_trainer_data: bool,
    use_hardwaired_4_screen_mirroring: bool,
    mapper_id: u16,
    submapper_id: u8,
    prg_wram_size: u32,
    prg_sram_size: u32,
    chr_wram_size: u32,
    chr_sram_size: u32,
}

impl INesHeader {
    fn from_bytes(mut header: [u8; 16]) -> Result<Self, CartridgeError> {
        // decode header
        Self::check_magic(&header[0..4])?;

        let prg_size_low = header[4] as u16;
        let chr_size_low = header[5] as u16;
        let is_chr_ram = chr_size_low == 0;

        let hardwired_mirroring_vertical = header[6] & 1 != 0;
        header[6] >>= 1;
        let has_prg_ram_battery = header[6] & 1 != 0;
        header[6] >>= 1;
        let contain_trainer_data = header[6] & 1 != 0;
        header[6] >>= 1;
        let use_hardwaired_4_screen_mirroring = header[6] & 1 != 0;
        header[6] >>= 1;
        let mapper_id_low = (header[6] & 0xF) as u16;

        let console_type = header[7] & 0x3;
        header[7] >>= 2;
        let is_nes_2 = (header[7] & 0x3) == 2;
        header[7] >>= 2;
        let mapper_id_middle = (header[7] & 0xF) as u16;

        if !is_nes_2 {
            let prg_ram_size = if header[8] == 0 { 1 } else { header[8] };
            let ntcs_tv_system = header[9] & 1 == 0;

            if header[9] >> 1 != 0 {
                return Err(CartridgeError::HeaderError);
            }

            let is_prg_ram_present = (header[10] >> 4) & 1 == 0;
            let board_has_bus_conflict = (header[10] >> 5) & 1 != 0;

            for i in 12..=15 {
                if header[i] != 0 {
                    return Err(CartridgeError::HeaderError);
                }
            }
            Ok(Self {
                prg_rom_size: prg_size_low,
                chr_rom_size: chr_size_low,
                is_chr_ram,
                hardwired_mirroring_vertical,
                has_prg_ram_battery,
                contain_trainer_data,
                use_hardwaired_4_screen_mirroring,
                mapper_id: mapper_id_middle << 4 | mapper_id_low,
                submapper_id: 0,
                prg_wram_size: prg_ram_size as u32 * 0x2000,
                prg_sram_size: prg_ram_size as u32 * 0x2000,
                chr_wram_size: 0x2000, // can only use 8kb
                chr_sram_size: 0x2000,
            })
        } else {
            let mapper_id_high = (header[8] & 0xF) as u16;
            header[8] >>= 4;
            let submapper_id = header[8] & 0xF;

            let prg_size_high = (header[9] & 0xF) as u16;
            let chr_size_high = ((header[9] >> 4) & 0xF) as u16;

            let shift_size = (header[10] & 0xF) as u32;
            let prg_wram_size_bytes = if shift_size != 0 { 64 << shift_size } else { 0 };
            header[10] >>= 4;
            let shift_size = (header[10] & 0xF) as u32;
            let prg_sram_size_bytes = if shift_size != 0 { 64 << shift_size } else { 0 };

            let shift_size = (header[11] & 0xF) as u32;
            let chr_wram_size_bytes = if shift_size != 0 { 64 << shift_size } else { 0 };
            header[11] >>= 4;
            let shift_size = (header[11] & 0xF) as u32;
            let chr_sram_size_bytes = if shift_size != 0 { 64 << shift_size } else { 0 };

            // TODO: implement the rest

            Ok(Self {
                prg_rom_size: prg_size_high << 8 | prg_size_low,
                chr_rom_size: chr_size_high << 8 | chr_size_low,
                is_chr_ram,
                hardwired_mirroring_vertical,
                has_prg_ram_battery,
                contain_trainer_data,
                use_hardwaired_4_screen_mirroring,
                mapper_id: mapper_id_high << 8 | mapper_id_middle << 4 | mapper_id_low,
                submapper_id,
                prg_wram_size: prg_wram_size_bytes,
                prg_sram_size: prg_sram_size_bytes,
                chr_wram_size: chr_wram_size_bytes,
                chr_sram_size: chr_sram_size_bytes,
            })
        }
    }

    fn empty() -> Self {
        Self::from_bytes([0x4E, 0x45, 0x53, 0x1A, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap()
    }

    fn check_magic(header: &[u8]) -> Result<(), CartridgeError> {
        let real = [0x4E, 0x45, 0x53, 0x1A];

        if header == real {
            Ok(())
        } else {
            Err(CartridgeError::HeaderError)
        }
    }
}

pub struct Cartridge {
    file_path: Box<Path>,
    header: INesHeader,

    _trainer_data: Vec<u8>,
    pub(crate) prg_data: Vec<u8>,
    pub(crate) chr_data: Vec<u8>,
    prg_ram_data: Vec<u8>,

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
                let header = INesHeader::from_bytes(header)?;

                let sram_data = if header.has_prg_ram_battery {
                    // try to load old save data
                    if let Ok(data) =
                        Self::load_sram_file(file_path.as_ref(), header.prg_sram_size as usize)
                    {
                        data
                    } else {
                        vec![0; header.prg_sram_size as usize]
                    }
                } else {
                    vec![0; header.prg_wram_size as usize]
                };

                println!("mapper {}", header.mapper_id);

                // initialize the mapper first, so that if it is not supported yet,
                // panic
                let mapper = Self::get_mapper(&header)?;

                let mut trainer_data = Vec::new();

                // read training data if present
                if header.contain_trainer_data {
                    trainer_data.resize(512, 0);
                    file.read_exact(&mut trainer_data)?;
                }

                // read PRG data
                let mut prg_data = vec![0; (header.prg_rom_size as usize) * 16 * 1024];
                file.read_exact(&mut prg_data)?;

                // read CHR data
                let chr_data = if !header.is_chr_ram {
                    let mut data = vec![0; (header.chr_rom_size as usize) * 8 * 1024];
                    file.read_exact(&mut data)?;

                    data
                } else {
                    // TODO: there is no way of knowing if we are using CHR WRAM or SRAM
                    let ram_size = header.chr_wram_size;

                    vec![0; ram_size as usize]
                };

                // there are missing parts
                let current = file.seek(SeekFrom::Current(0))?;
                let end = file.seek(SeekFrom::End(0))?;
                if current != end {
                    Err(CartridgeError::TooLargeFile(end - current))
                } else {
                    Ok(Self {
                        file_path: file_path.as_ref().to_path_buf().into_boxed_path(),
                        header,
                        _trainer_data: trainer_data,
                        prg_data,
                        chr_data,
                        prg_ram_data: sram_data,
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
            header: INesHeader::empty(),
            _trainer_data: Vec::new(),
            prg_data: Vec::new(),
            chr_data: Vec::new(),
            prg_ram_data: Vec::new(),
            mapper: Box::new(Mapper0::new()),

            is_empty: true,
        }
    }

    fn get_mapper(header: &INesHeader) -> Result<Box<dyn Mapper>, CartridgeError> {
        let mut mapper: Box<dyn Mapper> = match header.mapper_id {
            0 => Box::new(Mapper0::new()),
            1 => Box::new(Mapper1::new()),
            2 => Box::new(Mapper2::new()),
            3 => Box::new(Mapper3::new()),
            4 => Box::new(Mapper4::new()),
            7 => Box::new(Mapper7::new()),
            9 => Box::new(Mapper9::new()),
            10 => Box::new(Mapper10::new()),
            11 => Box::new(Mapper11::new()),
            66 => Box::new(Mapper66::new()),
            _ => {
                return Err(CartridgeError::MapperNotImplemented(header.mapper_id));
            }
        };

        // FIXME: fix parameters types to support INES2.0
        // should always call init in a new mapper, as it is the only way
        // they share a constructor
        mapper.init(
            header.prg_rom_size as u8,
            header.is_chr_ram,
            if !header.is_chr_ram {
                header.chr_rom_size as u8
            } else {
                (header.chr_wram_size / 0x2000) as u8
            },
            if header.has_prg_ram_battery {
                header.prg_sram_size / 0x2000
            } else {
                header.prg_wram_size / 0x2000
            } as u8,
        );

        Ok(mapper)
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

        let size = file.write(&self.prg_ram_data)?;

        if size != self.header.prg_sram_size as usize {
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
                    0x6000..=0x7FFF => *self
                        .prg_ram_data
                        .get(new_address)
                        .expect("SRAM out of bounds"),
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
                            .prg_ram_data
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

        if self.header.use_hardwaired_4_screen_mirroring {
            MirroringMode::FourScreen
        } else {
            if self.mapper.is_hardwired_mirrored() {
                if self.header.hardwired_mirroring_vertical {
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
        if !self.is_empty && self.header.has_prg_ram_battery {
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
