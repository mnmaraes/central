#[macro_use]
extern crate diesel;

mod ipc;
mod models;

pub mod model {
    pub use models::*;
}

pub mod command_client {
    pub use crate::ipc::{Create, Delete, NoteCommandClient, Update};
}

pub mod query_client {
    pub use crate::ipc::{GetContent, GetIndex, NoteQueryClient};
}

pub mod status_client {
    pub use crate::ipc::{Check, NoteStoreStatusClient};
}
