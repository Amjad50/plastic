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

    let mut nes = NES::new(&args[1], SfmlProvider {}).expect("");
    nes.run();
}
