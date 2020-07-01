mod nes;
use nes::NES;
use std::env::args;

fn main() {
    let args = args().collect::<Vec<String>>();

    if args.len() < 2 {
        eprintln!("USAGE: {} <rom-file>", args[0]);
        return;
    }

    let mut nes = NES::new(&args[1]).expect("");

    nes.run();
}
