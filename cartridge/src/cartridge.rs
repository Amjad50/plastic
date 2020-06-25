use super::{error::CartridgeError, mapper::Mapper};
use common::{Bus, Device};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

pub struct Cartridge {
    // header
    size_prg: u8,
    size_chr: u8,
    mapper_id: u8,
    mirroring_vertical: bool,
    contain_sram: bool,
    contain_trainer: bool,
    ignore_mirroring: bool,
    vs_unisystem: bool,        // don't know what is this (flag 7)
    _playchoice_10_hint: bool, // not used
    is_nes_2: bool,

    pub trainer_data: Vec<u8>,
    pub prg_data: Vec<u8>,
    pub chr_data: Vec<u8>,

    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    // TODO: not sure if it should consume the file or not
    pub fn from_file(mut file: File) -> Result<Self, CartridgeError> {
        let mut header = [0; 16];
        file.read_exact(&mut header)?;

        // decode header
        Cartridge::check_magic(&header[0..4])?;

        let size_prg = header[4];
        let size_chr = header[5];

        let mirroring_vertical = header[6] & 1 != 0;
        header[6] >>= 1;
        let contain_sram = header[6] & 1 != 0;
        header[6] >>= 1;
        let contain_trainer = header[6] & 1 != 0;
        header[6] >>= 1;
        let ignore_mirroring = header[6] & 1 != 0;
        header[6] >>= 1;
        let lower_mapper = header[6]; // the rest

        let vs_unisystem = header[7] & 1 != 0;
        header[7] >>= 1;
        let _playchoice_10_hint = header[7] & 1 != 0;
        header[7] >>= 1;
        let is_nes_2 = header[7] & 0b11 == 2;
        header[7] >>= 2;
        let upper_mapper = header[7]; // the rest

        let mapper_id = upper_mapper << 4 | lower_mapper;

        let mut trainer_data = Vec::new();

        // read training data if present
        if contain_trainer {
            trainer_data.resize(512, 0);
            file.read_exact(&mut trainer_data)?;
        }

        // read PRG data
        let mut prg_data = Vec::new();
        prg_data.resize((size_prg as usize) * 16 * 1024, 0);
        file.read_exact(&mut prg_data)?;

        // read CHR data
        let mut chr_data = Vec::new();
        chr_data.resize((size_chr as usize) * 8 * 1024, 0);
        file.read_exact(&mut chr_data)?;

        // there are missing parts
        if file.seek(SeekFrom::Current(0))? != file.seek(SeekFrom::End(0))? {
            Err(CartridgeError::TooLargeFile)
        } else {
            Ok(Self {
                size_prg,
                size_chr,
                mapper_id,
                mirroring_vertical,
                contain_sram,
                contain_trainer,
                ignore_mirroring,
                vs_unisystem,
                _playchoice_10_hint,
                is_nes_2,
                trainer_data,
                prg_data,
                chr_data,
                // TODO: remove after testing
                mapper: Box::new(super::mappers::Mapper0::new()),
            })
        }
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

impl Bus for Cartridge {
    // TODO: implement
    fn read(&self, address: u16, device: Device) -> u8 {
        let address = self.mapper.map(address);

        match device {
            // CPU is reading PRG only
            Device::CPU => *self
                .prg_data
                .get(address as usize)
                .expect("PRG out of bounds"),
            // PPU is reading CHR data
            Device::PPU => *self
                .chr_data
                .get(address as usize)
                .expect("CHR out of bounds"),
        }
    }
    fn write(&mut self, address: u16, data: u8, device: Device) {
        let address = self.mapper.map(address);

        // ## This is only a ROM data (read only) ##
        //
        // match device {
        //     Device::CPU => {
        //         *self
        //             .prg_data
        //             .get_mut(address as usize)
        //             .expect("PRG out of bounds") = data;
        //     }
        //     Device::PPU => {
        //         *self
        //             .chr_data
        //             .get_mut(address as usize)
        //             .expect("CHR out of bounds") = data;
        //     }
        // }
    }
}
