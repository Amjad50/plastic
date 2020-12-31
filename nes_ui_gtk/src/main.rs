mod ui;
use plastic_core::nes::NES;
use std::env::args;
use ui::GtkProvider;

fn main() {
    let args = args().collect::<Vec<String>>();

    let nes = if args.len() >= 2 {
        NES::new(&args[1], GtkProvider::new())
    } else {
        Ok(NES::new_without_file(GtkProvider::new()))
    };

    match nes {
        Ok(mut nes) => {
            nes.run();
        }
        Err(err) => {
            eprintln!("[ERROR] {}", err);
        }
    }
}
