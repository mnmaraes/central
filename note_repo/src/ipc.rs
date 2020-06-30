use std::env::{current_dir, set_current_dir, var};

use cliff::{client, router};

use actix::prelude::*;

use tracing::info;

use failure::{format_err, Error};

use crate::runners::*;
use models::{NoteDescriptor, NoteRef};

pub struct NoteRepo;

impl Actor for NoteRepo {
    type Context = Context<Self>;
}

impl Default for NoteRepo {
    fn default() -> Self {
        dotenv::dotenv().ok();

        let current = current_dir();
        let home = var("NOTE_HOME").map(|s| s.into()).or(current).unwrap();

        set_current_dir(home).ok();

        NoteRepo
    }
}

router! {
    NoteRepo;
    [
        NoteCommand [
            Create { reference: NoteRef, body: String } -> {
                if let NoteRef::Path(path) = reference {
                    info!("Creating note at {} with initial body {}", path, body);
                    // TODO

                }
            } => Success,
            Update { reference: NoteRef, body: String } -> {
                if let NoteRef::Path(path) = reference  {
                    info!("Updating note at {} with body {}", path, body);
                    // TODO
                }
            } => Success,
            Delete { reference: NoteRef } -> {
                if let NoteRef::Path(path) = reference {
                    info!("Deleting note at {}", path);
                    // TODO
                }
            } => Success,
        ],
        NoteQuery [
            GetContent { reference: NoteRef } -> {
                let content = if let NoteRef::Path(path) = reference {
                    get_note(path)
                } else {
                    Err(format_err!("Invalid Note reference"))
                };
            } => [
                let Err(e) = content => Error [String] { description: format!("{}", e) },
                => Content [String] { content: content.unwrap() },
            ],
            GetIndex -> {
                let index = get_all_descriptors();
                info!("Descriptors: {:?}", index);
            } => [
                let Err(e) = index => Error [String] { description: format!("{}", e) },
                => Index [Vec<NoteDescriptor>] { index: index.unwrap() }
            ],
        ],
        NoteRepoStatus [
            Check => Alive
        ]
    ]
}

client! {
    NoteCommand {
        actions => [
            Create { reference: NoteRef, body: String },
            Update { reference: NoteRef, body: String },
            Delete { reference: NoteRef },
        ],
    }
}

client! {
    NoteQuery {
        actions => [
            GetIndex wait Result<Vec<NoteDescriptor>, Error>,
            GetContent { reference: NoteRef } wait Result<String, Error>,
        ],
        response_mapping => [
            Index { index } => [
                Result<Vec<NoteDescriptor>, Error>: Ok(index)
            ],
            Content { content } => [
                Result<String, Error>: Ok(content)
            ],
            Error { description } => [
                Result<Vec<NoteDescriptor>, Error>: Err(format_err!("{}", description)),
                Result<String, Error>: Err(format_err!("{}", description))
            ]
        ]
    }
}

client! {
    NoteRepoStatus {
        actions => [
            Check wait,
        ],
        response_mapping => [
            Alive => [ () ]
        ]
    }
}
