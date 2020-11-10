mod nes_communication;
mod ui;

use nes_ui_base::nes::NES;
use ui::MobileProvider;

use nes_communication::NesRequest;
pub use nes_communication::{NesKey, NesRequestType, NesResponseType};

use allo_isolate::Isolate;
use lazy_static::lazy_static;
use std::os::raw::c_char;
use std::sync::mpsc::*;
use std::sync::Mutex;

lazy_static! {
    static ref SENDER: Mutex<Option<SyncSender<NesRequest>>> = Mutex::new(None);
}

pub(crate) fn send(event: NesRequest) {
    if let Ok(sender) = SENDER.lock() {
        if let Some(sender) = sender.as_ref() {
            sender.send(event).unwrap();
        }
    }
}

/// This is only here to add it to the binding.h file when building,
/// looks like if the enum is not used, it will not be generated
#[no_mangle]
pub extern "C" fn nes_response_empty_do_not_call(_: NesResponseType) {}

/// This is only here to add it to the binding.h file when building,
/// looks like if the enum is not used, it will not be generated
#[no_mangle]
pub extern "C" fn nes_key_empty_do_not_call(_: NesKey) {}

#[no_mangle]
pub extern "C" fn nes_sample_rate() -> u32 {
    nes_ui_base::nes_audio::SAMPLE_RATE
}

#[no_mangle]
pub extern "C" fn nes_request(event: NesRequestType, data: *const c_char) {
    // TODO: send errors to dart
    if let Ok(event) = NesRequest::from_nes_request(event, data) {
        send(event);
    }
}

#[no_mangle]
pub extern "C" fn run_nes(sending_port: i64) {
    let mut nes = None;
    if let Ok(mut sender) = SENDER.lock() {
        // only initialize if sender is None
        if sender.is_none() {
            // allow for 50 events before blocking
            let (tx, rx) = sync_channel(50);
            *sender = Some(tx);

            nes = Some(NES::new_without_file(MobileProvider::new(
                Isolate::new(sending_port),
                rx,
            )));
        }
    } else {
        // don't use [`send`], as it needs to lock SENDER, messy stuff XD
        Isolate::new(sending_port).post("ERROR SENDER is locked");
    }

    if let Some(mut nes) = nes {
        nes.run();
    } else {
        send(NesRequest::Log(
            "[ERROR]: tried to open a NES while one is already running".to_string(),
        ));
    }
}
