mod ui;
use plastic_core::nes::NES;
use std::env::args;

fn main() {
    let args = args().collect::<Vec<String>>();

    if args.len() < 2 {
        eprintln!("USAGE: {} <rom-file> [-a]\n-a: remove audio", args[0]);
        return;
    }

    let nes = match NES::new(&args[1]) {
        Ok(nes) => nes,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let has_audio = !(args.len() == 3 && args[2] == "-a");

    ui::Ui::new(nes, has_audio).run();
}
