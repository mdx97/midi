mod cli;
mod error;
mod midi;

use midi::MidiFile;

use crate::cli::parse_file_argument;
use crate::error::Error;

fn main() {
    if let Err(error) = run() {
        println!("{}", error);
    }
}

/// Primary entry point of the program.
fn run() -> Result<(), Error> {
    let file = parse_file_argument()?;
    let midi = MidiFile::read(&file)?;
    println!("{:#?}", midi);
    Ok(())
}
