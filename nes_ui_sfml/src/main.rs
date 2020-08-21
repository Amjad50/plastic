mod ui;
use nes_ui_base::nes::NES;
use std::env::args;
use ui::SfmlProvider;

fn main() {
    let args = args().collect::<Vec<String>>();

    if args.len() < 2 {
        eprintln!("USAGE: {} <rom-file>", args[0]);
        return;
    }

    match NES::new(&args[1], SfmlProvider {}) {
        Ok(mut nes) => {
            nes.run();
        }
        Err(err) => {
            eprintln!("[ERROR] {}", err);
        }
    }
}
