use crate::nes_communication::{NesRequest, NesResponse};
use allo_isolate::Isolate;
use nes_ui_base::{
    nes_controller::StandardNESControllerState, nes_display::Color as NESColor, BackendEvent,
    UiEvent, UiProvider,
};
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::time::Duration;

pub struct MobileProvider {
    port: Isolate,
    event_receiver: Receiver<NesRequest>,
    current_saves_present: Vec<u8>,
}

impl MobileProvider {
    pub(crate) fn new(port: Isolate, event_receiver: Receiver<NesRequest>) -> Self {
        port.post(NesResponse::Log("MobileProvider instantiated".to_string()));
        Self {
            port,
            event_receiver,
            current_saves_present: Vec::new(),
        }
    }
}

impl UiProvider for MobileProvider {
    fn get_tv_color_converter() -> fn(&NESColor) -> [u8; 4] {
        |color| [color.b, color.g, color.r, 0xFF]
    }

    fn run_ui_loop(
        &mut self,
        ui_to_nes_sender: Sender<UiEvent>,
        nes_to_ui_receiver: Receiver<BackendEvent>,
        image: Arc<Mutex<Vec<u8>>>,
        ctrl_state: Arc<Mutex<StandardNESControllerState>>,
    ) {
        self.port
            .post(NesResponse::Log("MobileProvider started loop".to_string()));

        loop {
            if let Ok(event) = self.event_receiver.recv_timeout(Duration::from_millis(10)) {
                // TODO: send all error messages to dart
                match event {
                    NesRequest::Log(msg) => {
                        self.port.post(NesResponse::Log(msg));
                    }
                    NesRequest::Reset => {
                        ui_to_nes_sender.send(UiEvent::Reset).unwrap();
                    }
                    NesRequest::Exit => {
                        ui_to_nes_sender.send(UiEvent::Exit).unwrap();
                        break;
                    }
                    NesRequest::Pause => {
                        ui_to_nes_sender.send(UiEvent::Pause).unwrap();
                    }
                    NesRequest::Resume => {
                        ui_to_nes_sender.send(UiEvent::Resume).unwrap();
                    }
                    NesRequest::ButtonPress(btn) => {
                        if let Ok(mut ctrl_state) = ctrl_state.lock() {
                            ctrl_state.press(btn);
                        }
                    }
                    NesRequest::ButtonRelease(btn) => {
                        if let Ok(mut ctrl_state) = ctrl_state.lock() {
                            ctrl_state.release(btn);
                        }
                    }
                    NesRequest::LoadState(index) => {
                        ui_to_nes_sender.send(UiEvent::LoadState(index)).unwrap();
                    }
                    NesRequest::SaveState(index) => {
                        ui_to_nes_sender.send(UiEvent::SaveState(index)).unwrap();
                    }
                    NesRequest::LoadRom(filename) => {
                        ui_to_nes_sender.send(UiEvent::LoadRom(filename)).unwrap();
                    }
                    NesRequest::GetImage => {
                        if let Ok(image) = image.lock() {
                            self.port.post(NesResponse::Image(image.to_vec()));
                        }
                    }
                    NesRequest::GetSavesPresent => {
                        self.port.post(NesResponse::SavesPresent(
                            self.current_saves_present.to_vec(),
                        ));
                    }
                }
            }

            if let Ok(event) = nes_to_ui_receiver.try_recv() {
                match event {
                    BackendEvent::PresentStates(states) => self.current_saves_present = states,
                    BackendEvent::Log(msg) => {
                        self.port
                            .post(NesResponse::Log(format!("NES backend: {}", msg)));
                    }
                    BackendEvent::AudioBuffer(buffer) => {
                        self.port.post(NesResponse::AudioBuffer(buffer));
                    }
                }
            }
        }

        self.port.post(NesResponse::Exit);
    }
}
