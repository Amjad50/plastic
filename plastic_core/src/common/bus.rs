#[derive(PartialEq, Clone, Copy)]
pub enum Device {
    CPU,
    PPU,
}

pub trait Bus {
    fn read(&self, address: u16, device: Device) -> u8;
    fn write(&mut self, address: u16, data: u8, device: Device);
}

/// macro used to generate binding for enum to convert it from u16
/// which will make generating memory mapped registers much easier
/// and its used in the PPU and APU
macro_rules! memory_mapped_registers {
    (($($vis:tt)*) enum $name:ident {$($field:ident =$expr:expr,)*}) => {
        $($vis)* enum $name {
            $($field =$expr,)*
        }

         impl std::convert::TryFrom<u16> for $name {
             type Error = ();

             fn try_from(v: u16) -> Result<Self, Self::Error> {
                 match v {
                     $(x if x == $name::$field as u16 => Ok($name::$field),)*
                     _ => Err(()),
                 }
             }
         }
    };

    (pub enum $name:ident {$($field:ident=$expr:expr,)*}) => {
        memory_mapped_registers! {(pub) enum $name {$($field = $expr,)*}}
    };

    (enum $name:ident {$($field:ident $(= $expr:expr)*,)*}) => {
        memory_mapped_registers! {() enum $name  {$($field =$expr,)*}}
    };
}
