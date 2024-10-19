mod ui;
use plastic_core::NES;
use std::env::args;

fn main() {
    let args = args().collect::<Vec<String>>();

    let mut file = args.get(1).map(|s| s.as_str());

    if file == Some("-h") || file == Some("--help") {
        eprintln!("USAGE: {} [rom-file] [-a]\n-a: remove audio", args[0]);
        return;
    }

    let mut has_audio = true;

    if file == Some("-a") {
        file = None;
        has_audio = false;
    }

    if has_audio && args.get(2).map(|s| s.as_str()) == Some("-a") {
        has_audio = false;
    }

    let nes = match file {
        Some(f) => NES::new(f),
        None => Ok(NES::new_without_file()),
    };
    let nes = match nes {
        Ok(nes) => nes,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    ui::Ui::new(nes, has_audio).run();
}
