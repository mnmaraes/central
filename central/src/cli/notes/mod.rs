use std::env::var;
use std::io::{BufReader, BufWriter, Read, Write};
use std::process::Command;

use clap::Clap;

use tempfile::Builder;

use dialoguer::Select;

use note_store::{
    Check, Create, Get, Note, NoteCommandClient, NoteQueryClient, NoteStoreStatusClient, Update,
};

registry::interface! {
    NoteCommand,
    NoteQuery,
    NoteStoreStatus
}

/// Manages the raw note data.
/// Subcommands: create
#[derive(Clap)]
pub struct NoteCommand {
    #[clap(subcommand)]
    subcmd: NoteCommands,
}

impl NoteCommand {
    pub fn run(&self) {
        self.subcmd.run();
    }
}

/// Creates a new note and saves it to central
#[derive(Clap)]
struct CreateNote {
    /// The temporary file extension to use for the creation file
    /// Helpful to enable editor extension based features (e.g.: Syntax highlighting)
    #[clap(short, long, default_value = "md")]
    extension: String,
}

impl CreateNote {
    fn run(&self) {
        let extension = format!(".{}", self.extension);
        let file = Builder::new()
            .suffix(&extension)
            .tempfile()
            .expect("Couldn't create a temporary file");

        let editor_cmd = match var("EDITOR") {
            Ok(editor) => editor,
            Err(_) => "vi".to_string(),
        };

        actix_rt::System::new("main").block_on(async move {
            let status_client = require::<NoteStoreStatusClient>().await.unwrap();
            status_client
                .send(Check)
                .await
                .expect("Couldn't contact Note Store. Make sure it is running and registered");

            Command::new(editor_cmd)
                .arg(file.path().to_str().unwrap())
                .status()
                .expect("Failed to start editor");

            let mut contents = String::new();
            BufReader::new(file.into_file())
                .read_to_string(&mut contents)
                .expect("Couldn't read file");

            let note_client = require::<NoteCommandClient>().await.unwrap();
            note_client
                .send(Create { body: contents })
                .await
                .expect("Failed To Notify Note Sore");
        });
    }
}

/// Selects and updates an existing note
#[derive(Clap)]
struct UpdateNote {
    /// The temporary file extension to use for the creation file
    /// Helpful to enable editor extension based features (e.g.: Syntax highlighting)
    #[clap(short, long, default_value = "md")]
    extension: String,
}

impl UpdateNote {
    fn run(&self) {
        let extension = format!(".{}", self.extension);
        let mut file = Builder::new()
            .suffix(&extension)
            .tempfile()
            .expect("Couldn't create a temporary file");

        let editor_cmd = match var("EDITOR") {
            Ok(editor) => editor,
            Err(_) => "vi".to_string(),
        };

        actix_rt::System::new("main").block_on(async move {
            let query_client = require::<NoteQueryClient>().await.unwrap();
            let notes: Vec<Note> = query_client
                .send(Get)
                .await
                .expect("Couldn't fetch existing notes")
                .expect("Couldn't fetch existing notes");

            let first_lines: Vec<_> = notes
                .iter()
                .filter_map(|note| note.body.lines().next())
                .collect();

            let selection = Select::new()
                .with_prompt("Select note to update:")
                .items(&first_lines)
                .interact()
                .unwrap();

            BufWriter::new(file.as_file_mut())
                .write_all(notes[selection].body.as_bytes())
                .expect("Couldn't write note to file");

            Command::new(editor_cmd)
                .arg(file.path().to_str().unwrap())
                .status()
                .expect("Failed to start editor");

            let mut contents = String::new();
            BufReader::new(file.reopen().unwrap())
                .read_to_string(&mut contents)
                .expect("Couldn't read file");

            println!("{}", contents);

            let note_client = require::<NoteCommandClient>().await.unwrap();
            note_client
                .send(Update {
                    id: notes[selection].id.to_string(),
                    body: contents,
                })
                .await
                .expect("Failed To Notify Note Sore");
        });
    }
}

#[derive(Clap)]
enum NoteCommands {
    Update(UpdateNote),
    New(CreateNote),
}

impl NoteCommands {
    fn run(&self) {
        use NoteCommands::*;

        match self {
            Update(update) => update.run(),
            New(create_note) => create_note.run(),
        };
    }
}
