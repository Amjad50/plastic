#[cfg(target_os = "windows")]
mod ui;

#[cfg(target_os = "windows")]
fn main() {
    use nes_ui_base::nes::NES;
    use std::env::args;
    use ui::NwgProvider;

    let args = args().collect::<Vec<String>>();

    let nes = if args.len() >= 2 {
        NES::new(&args[1], NwgProvider::new())
    } else {
        Ok(NES::new_without_file(NwgProvider::new()))
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

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This package can only be compiled to windows");
}
