use allo_isolate::ffi::DartCObject;
use allo_isolate::IntoDart;
use nes_ui_base::nes_controller::StandardNESKey;
use std::convert::TryFrom;
use std::ffi::CStr;
use std::os::raw::c_char;

#[repr(C)]
pub enum NesRequestType {
    // no args
    Reset,
    Exit,
    Pause,
    Resume,
    GetImage,
    GetSavesPresent,
    // one u8 arg
    ButtonPress,
    ButtonRelease,
    LoadState,
    SaveState,
    // one string (char*) arg
    LoadRom,
}

/// using [`NesRequest::from_nes_request`] to convert [`NesRequestType`]
/// into a better enum to deal with
#[derive(Debug)]
pub(crate) enum NesRequest {
    // no args
    Reset,
    Exit,
    Pause,
    Resume,
    GetImage,
    GetSavesPresent,
    // one u8 arg
    ButtonPress(StandardNESKey),
    ButtonRelease(StandardNESKey),
    LoadState(u8),
    SaveState(u8),
    // one string (char*) arg
    LoadRom(String),
    // this is a spacial type, not present for outside, used to allow sending prints,
    // MobileProvider should handle this properly
    Log(String),
}

// this is for outside
#[repr(C)]
pub enum NesKey {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl TryFrom<u8> for NesKey {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == NesKey::A as u8 => Ok(NesKey::A),
            x if x == NesKey::B as u8 => Ok(NesKey::B),
            x if x == NesKey::Select as u8 => Ok(NesKey::Select),
            x if x == NesKey::Start as u8 => Ok(NesKey::Start),
            x if x == NesKey::Up as u8 => Ok(NesKey::Up),
            x if x == NesKey::Down as u8 => Ok(NesKey::Down),
            x if x == NesKey::Left as u8 => Ok(NesKey::Left),
            x if x == NesKey::Right as u8 => Ok(NesKey::Right),
            _ => Err(()),
        }
    }
}

impl From<NesKey> for StandardNESKey {
    fn from(key: NesKey) -> Self {
        match key {
            NesKey::A => StandardNESKey::A,
            NesKey::B => StandardNESKey::B,
            NesKey::Select => StandardNESKey::Select,
            NesKey::Start => StandardNESKey::Start,
            NesKey::Up => StandardNESKey::Up,
            NesKey::Down => StandardNESKey::Down,
            NesKey::Left => StandardNESKey::Left,
            NesKey::Right => StandardNESKey::Right,
        }
    }
}

impl NesRequest {
    // TODO: fix error
    pub(crate) fn from_nes_request(event: NesRequestType, data: *const c_char) -> Result<Self, ()> {
        match event {
            NesRequestType::Reset => Ok(NesRequest::Reset),
            NesRequestType::Exit => Ok(NesRequest::Exit),
            NesRequestType::Pause => Ok(NesRequest::Pause),
            NesRequestType::Resume => Ok(NesRequest::Resume),
            NesRequestType::GetImage => Ok(NesRequest::GetImage),
            NesRequestType::GetSavesPresent => Ok(NesRequest::GetSavesPresent),
            NesRequestType::ButtonPress => {
                if let Ok(key) = NesKey::try_from(data as u8) {
                    Ok(NesRequest::ButtonPress(key.into()))
                } else {
                    Err(())
                }
            }
            NesRequestType::ButtonRelease => {
                if let Ok(key) = NesKey::try_from(data as u8) {
                    Ok(NesRequest::ButtonRelease(key.into()))
                } else {
                    Err(())
                }
            }
            NesRequestType::LoadState => Ok(NesRequest::LoadState(data as u8)),
            NesRequestType::SaveState => Ok(NesRequest::SaveState(data as u8)),
            NesRequestType::LoadRom => {
                if data.is_null() {
                    Err(())
                } else {
                    // SAFETY: nothing for now
                    if let Ok(cstr) = unsafe { CStr::from_ptr(data) }.to_str() {
                        Ok(NesRequest::LoadRom(cstr.to_string()))
                    } else {
                        Err(())
                    }
                }
            }
        }
    }
}

#[repr(C)]
pub enum NesResponseType {
    // no args
    ExitResponse,
    // vec u8 arg
    Image,
    SavesPresent,
    // logging
    Log,
}

/// using [`NesRequest::from_nes_request`] to convert [`NesRequestType`]
/// into a better enum to deal with
#[derive(Debug)]
pub(crate) enum NesResponse {
    // no args
    Exit,
    // vec u8 arg
    Image(Vec<u8>),
    SavesPresent(Vec<u8>),
    // string
    Log(String),
}

impl NesResponse {
    pub(crate) fn to_nes_response_type_and_data(self) -> (NesResponseType, Vec<u8>) {
        match self {
            NesResponse::Exit => (NesResponseType::ExitResponse, Vec::with_capacity(0)),
            NesResponse::Image(data) => (NesResponseType::Image, data),
            NesResponse::SavesPresent(data) => (NesResponseType::SavesPresent, data),
            NesResponse::Log(data) => (
                NesResponseType::Log,
                format!("libplastic_mobile [LOG]: {}", data).into_bytes(),
            ),
        }
    }
}

impl IntoDart for NesResponse {
    fn into_dart(self) -> DartCObject {
        let (ty, mut data) = self.to_nes_response_type_and_data();
        let mut result = Vec::with_capacity(data.len() + 1);
        result.push(ty as u8);
        result.append(&mut data);

        result.into_dart()
    }
}
