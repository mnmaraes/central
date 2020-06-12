#[macro_use]
extern crate diesel;

mod models;
mod schema;

use models::{create_note, delete_note, get_all, update_note, Note};

use macros::ipc;

ipc! {
    Note {
        command => [
            Create { body: String } -> {
                create_note(&self.connection, &body)
            },
            Update { id: String, body: String } -> {
                update_note(&self.connection, &id, &body)
            },
            Delete { id: String } -> {
                delete_note(&self.connection, &id)
            }
        ],
        query => [
            Get -> {
                get_all(&self.connection)
            } into Notes { notes: Vec<Note> = result.unwrap() } => {
                notes
            } as Vec<Note>
        ]
    }
}

pub mod model {
    pub use crate::models::Note;
}

pub mod command_client {
    pub use crate::ipc::{Create, Delete, NoteCommandClient, Update};
}

pub mod query_client {
    pub use crate::ipc::{Get, NoteQueryClient};
}

pub mod status_client {
    pub use crate::ipc::{Check, NoteStoreStatusClient};
}
