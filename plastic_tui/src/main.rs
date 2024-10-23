// Import the ui module and NES emulator from plastic_core, as well as argument
mod ui;
use plastic_core::NES;
use std::env::args;

fn main() {
    // Collect cl arguments into a vector
    let args = args().collect::<Vec<String>>();

    // Retrieve the ROM file (if provided)
    let mut file = args.get(1).map(|s| s.as_str());

    // Display usage information if the -h or --help flag is passed
    if file == Some("-h") || file == Some("--help") {
        eprintln!("USAGE: {} [rom-file] [-a]\n-a: remove audio", args[0]);
        return;
    }

    // Assume audio is enabled unless overridden
    let mut has_audio = true;

    // If the -a flag is passed without a ROM file, disable audio and clear file
    if file == Some("-a") {
        file = None;
        has_audio = false;
    }

    // If the second argument is -a, disable audio
    if has_audio && args.get(2).map(|s| s.as_str()) == Some("-a") {
        has_audio = false;
    }

    // Attempt to initialize the NES emulator with the provided ROM file, or without one
    let nes = match file {
        Some(f) => NES::new(f),       // Load the specified ROM file
        None => Ok(NES::new_without_file()), // No ROM file, start without loading a game
    };
    let nes = match nes {
        Ok(nes) => nes,               // Successfully created the NES instance
        Err(e) => {
            // Print an error message and exit if NES initialization fails
            eprintln!("Error: {}", e);
            return;
        }
    };

    // Start the UI with the initialized NES and the audio flag
    ui::Ui::new(nes, has_audio).run();
}
