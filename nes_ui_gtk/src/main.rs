mod ui;
use nes_ui_base::nes::NES;
use std::env::args;
use ui::GtkProvider;

fn main() {
    let args = args().collect::<Vec<String>>();

    let mut nes = if args.len() >= 2 {
        NES::new(&args[1], GtkProvider {}).expect("")
    } else {
        NES::new_without_file(GtkProvider {})
    };

    nes.run();
}
