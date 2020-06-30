use std::env::current_dir;
use std::path::Path;

use std::fs::{read_to_string, File};
use std::io::{BufRead, BufReader};
use std::process::Command;

use tracing::info;

use models::{NoteDescriptor, NoteRef};

use failure::Error;

pub fn get_note(path: String) -> Result<String, Error> {
    Ok(read_to_string(path)?)
}

pub fn get_all_descriptors() -> Result<Vec<NoteDescriptor>, Error> {
    info!("Working from: {:?}", current_dir());
    let ls = Command::new("fd")
        .args(&["-e", "md", "-c", "never"])
        .output()?;

    Ok(ls
        .stdout
        .lines()
        .filter_map(|res| res.ok())
        .map(|path| {
            let default = "".to_string();
            let title = read_heading(path.clone()).unwrap_or(default);

            NoteDescriptor {
                title,
                reference: NoteRef::Path(path),
            }
        })
        .collect())
}

fn read_heading<P>(path: P) -> Result<String, Error>
where
    P: AsRef<Path>,
{
    let file = File::open(path)?;
    let default: String = "".into();

    Ok(BufReader::new(file)
        .lines()
        .next()
        .unwrap_or(Ok(default))?
        .replace("#", "")
        .trim()
        .into())
}
