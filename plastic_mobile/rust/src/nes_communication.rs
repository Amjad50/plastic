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

impl std::convert::TryFrom<i32> for NesRequestType {
    type Error = ();
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == NesRequestType::Reset as i32 => Ok(NesRequestType::Reset),
            x if x == NesRequestType::Exit as i32 => Ok(NesRequestType::Exit),
            x if x == NesRequestType::Pause as i32 => Ok(NesRequestType::Pause),
            x if x == NesRequestType::Resume as i32 => Ok(NesRequestType::Resume),
            x if x == NesRequestType::GetImage as i32 => Ok(NesRequestType::GetImage),
            x if x == NesRequestType::GetSavesPresent as i32 => Ok(NesRequestType::GetSavesPresent),
            x if x == NesRequestType::ButtonPress as i32 => Ok(NesRequestType::ButtonPress),
            x if x == NesRequestType::ButtonRelease as i32 => Ok(NesRequestType::ButtonRelease),
            x if x == NesRequestType::LoadState as i32 => Ok(NesRequestType::LoadState),
            x if x == NesRequestType::SaveState as i32 => Ok(NesRequestType::SaveState),
            x if x == NesRequestType::LoadRom as i32 => Ok(NesRequestType::LoadRom),
            _ => Err(()),
        }
    }
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
    // vec f32
    AudioBuffer,
    // logging
    Log,
}

enum Either<TA, TB> {
    A(TA),
    B(TB),
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
    // vec f32 arg
    AudioBuffer(Vec<f32>),
    // string
    Log(String),
}

impl NesResponse {
    fn to_nes_response_type_and_data(self) -> (NesResponseType, Either<Vec<u8>, Vec<f32>>) {
        match self {
            NesResponse::Exit => (
                NesResponseType::ExitResponse,
                Either::A(Vec::with_capacity(0)),
            ),
            NesResponse::Image(data) => (NesResponseType::Image, Either::A(data)),
            NesResponse::SavesPresent(data) => (NesResponseType::SavesPresent, Either::A(data)),
            NesResponse::Log(data) => (
                NesResponseType::Log,
                Either::A(format!("libplastic_mobile [LOG]: {}", data).into_bytes()),
            ),
            NesResponse::AudioBuffer(data) => (NesResponseType::AudioBuffer, Either::B(data)),
        }
    }
}

impl IntoDart for NesResponse {
    fn into_dart(self) -> DartCObject {
        // the manual types here is just to make sure the second either is Vec<f32>,
        // as this is a SAFETY concerns for below
        let (ty, data): (_, Either<_, Vec<f32>>) = self.to_nes_response_type_and_data();

        let mut data = match data {
            Either::A(d) => d,
            Either::B(d) => {
                let d = d
                    .iter()
                    .map(|e| (e * std::i16::MAX as f32) as i16)
                    .collect::<Vec<i16>>();
                // SAFETY: `d` is [i16] here, when converted to [u8], every 1 elemnt
                // will be converted into 2 `u8` integers, _pre and _post will
                // be empty,
                // This is safe because u8 has size of 1 and alignment of 1,
                // i16 has size of 2 and alignment of 2, which can be divided into
                // u8 without problems
                let (pre, middle, post) = unsafe { d.align_to::<u8>() };
                // just to make sure
                assert!(pre.is_empty() && post.is_empty());
                middle.to_vec()
            }
        };

        let mut result = Vec::with_capacity(data.len() + 1);
        result.push(ty as u8);
        result.append(&mut data);
        result.into_dart()
    }
}
