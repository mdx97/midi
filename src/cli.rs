use std::path::PathBuf;

use crate::error::Error;

/// Attempts to get the provided file path from the command line arguments.
pub fn parse_file_argument() -> Result<PathBuf, Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err(Error::Usage);
    }

    let file = PathBuf::from(&args[1]);
    if !file.exists() {
        return Err(Error::general("provided file path does not exist"));
    }

    Ok(file)
}
