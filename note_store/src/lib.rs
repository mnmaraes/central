#[macro_use]
extern crate diesel;

mod ipc;
mod models;
mod schema;

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
