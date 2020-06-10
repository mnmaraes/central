#[macro_use]
extern crate diesel;

mod ipc;
mod models;
mod schema;

pub use ipc::{
    Check, Create, Delete, Get, NoteCommandClient, NoteQueryClient, NoteStoreStatusClient, Update,
};
pub use models::Note;
