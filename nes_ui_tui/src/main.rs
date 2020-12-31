mod event;
mod ui;
use plastic_core::nes::NES;
use std::env::args;
use ui::TuiProvider;

fn main() {
    let args = args().collect::<Vec<String>>();

    if args.len() < 2 {
        eprintln!("USAGE: {} <rom-file>", args[0]);
        return;
    }

    match NES::new(&args[1], TuiProvider {}) {
        Ok(mut nes) => {
            nes.run();
        }
        Err(err) => {
            eprintln!("[ERROR] {}", err);
        }
    }
}
