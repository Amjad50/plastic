mod ui;
use nes_ui_base::nes::NES;
use std::env::args;
use ui::GtkProvider;

fn main() {
    let args = args().collect::<Vec<String>>();

    if args.len() < 2 {
        eprintln!("USAGE: {} <rom-file>", args[0]);
        return;
    }

    let mut nes = NES::new(&args[1], GtkProvider {}).expect("");

    nes.run();
}
