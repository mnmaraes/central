use std::env::var;
use std::io::Read;
use std::process::Command;

use clap::Clap;

use tempfile::Builder;

use note_store::{Check, Create, NoteCommandClient, StatusClient};

registry::interface! {
    NoteCommand,
    Status
}

/// Manages the raw note data.
/// Subcommands: create
#[derive(Clap)]
pub struct Note {
    #[clap(subcommand)]
    subcmd: NoteCommands,
}

impl Note {
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
            // Check NoteStore status before writing our note
            let status_client = require::<StatusClient>().await.unwrap();
            status_client
                .send(Check)
                .await
                .expect("Couldn't contact Note Store. Make sure it is running and registered");

            Command::new(editor_cmd)
                .arg(file.path().to_str().unwrap())
                .status()
                .expect("Failed to start editor");

            let mut contents = String::new();
            file.into_file()
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

#[derive(Clap)]
enum NoteCommands {
    New(CreateNote),
}

impl NoteCommands {
    fn run(&self) {
        use NoteCommands::*;

        match self {
            New(create_note) => create_note.run(),
        };
    }
}
