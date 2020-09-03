use super::{
    error::{CartridgeError, SramError},
    mapper::{BankMapping, BankMappingType, Mapper},
    mappers::*,
};
use common::{interconnection::CpuIrqProvider, Bus, Device, MirroringMode, MirroringProvider};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::RangeInclusive,
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

        let mut console_type = header[7] & 0x3;
        header[7] >>= 2;
        let ines_2_ident = header[7] & 0x3;
        header[7] >>= 2;
        let mut mapper_id_middle = (header[7] & 0xF) as u16;

        if ines_2_ident == 0 {
            let mut is_archaic_ines = false;
            for i in 12..=15 {
                if header[i] != 0 {
                    is_archaic_ines = true;
                }
            }

            let prg_ram_size;

            if !is_archaic_ines {
                prg_ram_size = if header[8] == 0 { 1 } else { header[8] };
                let ntcs_tv_system = header[9] & 1 == 0;

                if header[9] >> 1 != 0 {
                    return Err(CartridgeError::HeaderError);
                }

                let is_prg_ram_present = (header[10] >> 4) & 1 == 0;
                let board_has_bus_conflict = (header[10] >> 5) & 1 != 0;
            } else {
                // ignore `header[7]` data
                console_type = 0;
                mapper_id_middle = 0;

                prg_ram_size = 1;
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

    /// returns the number of banks
    fn get_active_chr_data_size(&self, chr_bank_size: u16) -> u16 {
        if !self.is_chr_ram {
            (self.chr_rom_size as u32 * 0x2000 / chr_bank_size as u32) as u16
        } else {
            (self.chr_wram_size / chr_bank_size as u32) as u16
        }
    }

    /// returns the number of banks
    fn get_active_prg_ram_size(&self, prg_ram_bank_size: u16) -> u16 {
        ((if self.has_prg_ram_battery {
            self.prg_sram_size
        } else {
            self.prg_wram_size
        }) / prg_ram_bank_size as u32) as u16
    }

    /// returns the number of banks
    fn get_active_prg_rom_size(&self, prg_rom_bank_size: u16) -> u16 {
        (self.prg_rom_size as u32 * 0x4000 / prg_rom_bank_size as u32) as u16
    }
}

pub struct Cartridge {
    file_path: Box<Path>,
    header: INesHeader,

    _trainer_data: Vec<u8>,
    pub(crate) prg_data: Vec<u8>,
    pub(crate) chr_data: Vec<u8>,
    prg_ram_data: Vec<u8>,

    /// mapping of range 0x6000-0x7FFF, divided by the smallest block offered
    /// by the mapper
    cpu_ram_memory_mapping: Vec<BankMapping>,

    /// mapping of range 0x8000-0xFFFF, divided by the smallest block offered
    /// by the mapper, note that this can be mapped to RAM in some mappers
    /// like MMC5, but only part of it
    cpu_rom_memory_mapping: Vec<BankMapping>,

    /// mapping of range 0x0000-0x1FFFF, divided by the smallest block offered
    /// by the mapper
    ppu_memory_mapping: Vec<BankMapping>,

    mapper: Box<dyn Mapper>,

    /// cached here for faster usage
    mapper_register_write_range: RangeInclusive<u16>,

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
                let (mapper, initial_bank_mappings) = Self::get_mapper(&header)?;

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
                    // these should be overriden all in the call to `apply_bank_mapping`
                    let cpu_ram_memory_mapping = vec![
                        BankMapping {
                            ty: BankMappingType::CpuRam,
                            to: 0,
                            read: false,
                            write: false,
                        };
                        0x2000 / mapper.cpu_ram_bank_size() as usize
                    ];
                    let cpu_rom_memory_mapping = vec![
                        BankMapping {
                            ty: BankMappingType::CpuRom,
                            to: 0,
                            read: false,
                            write: false,
                        };
                        0x8000 / mapper.cpu_rom_bank_size() as usize
                    ];
                    let ppu_memory_mapping = vec![
                        BankMapping {
                            ty: BankMappingType::Ppu,
                            to: 0,
                            read: false,
                            write: false,
                        };
                        0x2000 / mapper.ppu_bank_size() as usize
                    ];

                    let mut cartridge = Self {
                        file_path: file_path.as_ref().to_path_buf().into_boxed_path(),
                        header,
                        _trainer_data: trainer_data,
                        prg_data,
                        chr_data,
                        prg_ram_data: sram_data,
                        cpu_ram_memory_mapping,
                        cpu_rom_memory_mapping,
                        ppu_memory_mapping,
                        mapper_register_write_range: mapper.registers_memory_range(),
                        mapper,

                        is_empty: false,
                    };

                    cartridge.apply_bank_mapping(initial_bank_mappings);

                    Ok(cartridge)
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

            cpu_ram_memory_mapping: Vec::new(),
            cpu_rom_memory_mapping: Vec::new(),
            ppu_memory_mapping: Vec::new(),

            mapper: Box::new(Mapper0::new()),

            mapper_register_write_range: 2..=1,

            is_empty: true,
        }
    }

    fn get_mapper(
        header: &INesHeader,
    ) -> Result<(Box<dyn Mapper>, Vec<(BankMappingType, u8, BankMapping)>), CartridgeError> {
        let mut mapper: Box<dyn Mapper> = match header.mapper_id {
            0 => Box::new(Mapper0::new()),
            1 => Box::new(Mapper1::new()),
            // 2 => Box::new(Mapper2::new()),
            // 3 => Box::new(Mapper3::new()),
            // 4 => Box::new(Mapper4::new()),
            // 7 => Box::new(Mapper7::new()),
            // 9 => Box::new(Mapper9::new()),
            // 10 => Box::new(Mapper10::new()),
            // 11 => Box::new(Mapper11::new()),
            // 12 => Box::new(Mapper12::new()),
            // 66 => Box::new(Mapper66::new()),
            _ => {
                return Err(CartridgeError::MapperNotImplemented(header.mapper_id));
            }
        };

        // FIXME: fix parameters types to support INES2.0
        // should always call init in a new mapper, as it is the only way
        // they share a constructor
        let initial_bank_mappings = mapper.init(
            header.get_active_prg_rom_size(mapper.cpu_rom_bank_size()),
            header.is_chr_ram,
            header.get_active_chr_data_size(mapper.ppu_bank_size()),
            header.get_active_prg_ram_size(mapper.cpu_ram_bank_size()),
        );

        Ok((mapper, initial_bank_mappings))
    }

    fn apply_bank_mapping(&mut self, bank_mappings: Vec<(BankMappingType, u8, BankMapping)>) {
        for (memory_type, index, mapping) in bank_mappings {
            let index = index as usize;
            match memory_type {
                BankMappingType::CpuRam => self.cpu_ram_memory_mapping[index] = mapping,
                BankMappingType::CpuRom => self.cpu_rom_memory_mapping[index] = mapping,
                BankMappingType::Ppu => self.ppu_memory_mapping[index] = mapping,
            }
        }
    }

    fn map_address(&self, address: u16, device: Device) -> (BankMapping, usize) {
        let (bank_mapping, offset_address) = match device {
            Device::CPU => match address {
                0x6000..=0x7FFF => {
                    let address_offset = address as usize & 0x1FFF;
                    let bank_size = self.mapper.cpu_ram_bank_size() as usize;
                    let mapping_index = address_offset / bank_size;
                    let offset_to_bank = address_offset & (bank_size - 1);
                    let bank = self.cpu_ram_memory_mapping[mapping_index];

                    (bank, offset_to_bank)
                }
                0x8000..=0xFFFF => {
                    let address_offset = address as usize & 0x7FFF;
                    let bank_size = self.mapper.cpu_rom_bank_size() as usize;
                    let mapping_index = address_offset / bank_size;
                    let offset_to_bank = address_offset & (bank_size - 1);
                    let bank = self.cpu_rom_memory_mapping[mapping_index];

                    (bank, offset_to_bank)
                }
                0x4200..=0x5FFF => {
                    // not used now
                    (
                        // just for the return value
                        BankMapping {
                            ty: BankMappingType::CpuRam,
                            to: 0,
                            read: false,
                            write: false,
                        },
                        0,
                    )
                }
                _ => unreachable!(),
            },
            Device::PPU => match address {
                0x0000..=0x1FFF => {
                    let address_offset = address as usize & 0x1FFF;
                    let bank_size = self.mapper.ppu_bank_size() as usize;
                    let mapping_index = address_offset / bank_size;
                    let offset_to_bank = address_offset & (bank_size - 1);
                    let bank = self.ppu_memory_mapping[mapping_index];

                    (bank, offset_to_bank)
                }
                _ => unreachable!(),
            },
        };

        let (bank_count, bank_size) = match bank_mapping.ty {
            BankMappingType::CpuRam => (
                self.header
                    .get_active_prg_ram_size(self.mapper.cpu_ram_bank_size()),
                self.mapper.cpu_ram_bank_size(),
            ),
            BankMappingType::CpuRom => (
                self.header
                    .get_active_prg_rom_size(self.mapper.cpu_rom_bank_size()),
                self.mapper.cpu_rom_bank_size(),
            ),
            BankMappingType::Ppu => (
                self.header
                    .get_active_chr_data_size(self.mapper.ppu_bank_size()),
                self.mapper.ppu_bank_size(),
            ),
        };

        let bank = if bank_count != 0 {
            bank_mapping.to as usize % bank_count as usize
        } else {
            0
        };
        let new_address = (bank * bank_size as usize) + offset_address;

        (bank_mapping, new_address)
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

        let (bank_mapping, new_address) = self.map_address(address, device);

        if bank_mapping.read {
            match bank_mapping.ty {
                BankMappingType::CpuRam => self.prg_ram_data[new_address],
                BankMappingType::CpuRom => self.prg_data[new_address],
                BankMappingType::Ppu => self.chr_data[new_address],
            }
        } else {
            0
        }
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        if self.is_empty {
            return;
        }

        if self.mapper_register_write_range.contains(&address) {
            let mapping_result = self.mapper.write_register(address, data);
            self.apply_bank_mapping(mapping_result);
        } else {
            let (bank_mapping, new_address) = self.map_address(address, device);

            if bank_mapping.write {
                let result = match bank_mapping.ty {
                    BankMappingType::CpuRam => &mut self.prg_ram_data[new_address],
                    BankMappingType::CpuRom => &mut self.prg_data[new_address],
                    BankMappingType::Ppu => &mut self.chr_data[new_address],
                };

                *result = data;
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
