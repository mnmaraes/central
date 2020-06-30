use std::env::var;
use std::io::{BufReader, BufWriter, Read, Write};
use std::process::Command;

use actix::Addr;

use failure::Error;

use dialoguer::Select;

use tempfile::{Builder, NamedTempFile};

use note_store::model::{NoteDescriptor, NoteRef};
use note_store::query_client::{GetContent, GetIndex, NoteQueryClient};

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

pub async fn get_notes() -> Result<Vec<NoteDescriptor>, Error> {
    let query_client = require::<NoteQueryClient>().await?;

    get_notes_from_client(&query_client).await
}

async fn get_notes_from_client(
    client: &Addr<NoteQueryClient>,
) -> Result<Vec<NoteDescriptor>, Error> {
    Ok(client.send(GetIndex).await??)
}

pub async fn select_note() -> Result<(NoteRef, String), Error> {
    let query_client = require::<NoteQueryClient>().await?;

    let notes: Vec<NoteDescriptor> = get_notes_from_client(&query_client).await?;

    let first_lines: Vec<_> = notes.iter().map(|note| note.title.clone()).collect();

    let selection = Select::new()
        .default(0)
        .with_prompt("Select note:")
        .items(&first_lines)
        .interact()?;

    let NoteDescriptor { reference, .. } = notes[selection].clone();
    let body = query_client
        .send(GetContent {
            reference: reference.clone(),
        })
        .await??;

    Ok((reference, body))
}
