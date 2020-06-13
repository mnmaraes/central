use std::env::var;
use std::io::{BufReader, BufWriter, Read, Write};
use std::process::Command;

use failure::Error;

use dialoguer::Select;

use tempfile::{Builder, NamedTempFile};

use note_store::model::Note;
use note_store::query_client::{Get, NoteQueryClient};

registry::interface! {
    NoteQuery,
}

pub struct TmpEditor {
    file: NamedTempFile,
}

impl TmpEditor {
    pub fn new() -> Self {
        Self {
            file: Builder::new()
                .suffix(".md")
                .tempfile()
                .expect("Couldn't create a temporary file"),
        }
    }

    pub fn open(&self) -> Result<(), Error> {
        let editor_cmd = match var("EDITOR") {
            Ok(editor) => editor,
            Err(_) => "vi".to_string(),
        };

        Command::new(editor_cmd)
            .arg(self.file.path().to_str().unwrap())
            .status()?;

        Ok(())
    }

    pub fn read_contents_to_string(&self, str: &mut String) -> Result<(), Error> {
        BufReader::new(self.file.reopen()?).read_to_string(str)?;

        Ok(())
    }

    pub fn load_contents(&mut self, str: &str) -> Result<(), Error> {
        BufWriter::new(self.file.as_file_mut()).write_all(str.as_bytes())?;

        Ok(())
    }
}

pub async fn get_notes() -> Result<Vec<Note>, Error> {
    let query_client = require::<NoteQueryClient>().await?;

    Ok(query_client.send(Get).await??)
}

pub async fn select_note() -> Result<Note, Error> {
    let notes: Vec<Note> = get_notes().await?;

    let first_lines: Vec<_> = notes
        .iter()
        .filter_map(|note| note.body.lines().next())
        .collect();

    let selection = Select::new()
        .with_prompt("Select note to delete:")
        .items(&first_lines)
        .interact()?;

    Ok(notes[selection].clone())
}
