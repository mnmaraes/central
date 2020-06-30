mod ipc;
mod runners;

pub mod model {
    pub use models::Note;
}

pub mod command_client {
    pub use crate::ipc::{Create, Delete, NoteCommandClient, Update};
}

pub mod query_client {
    pub use crate::ipc::{GetContent, GetIndex, NoteQueryClient};
}

pub mod status_client {
    pub use crate::ipc::{Check, NoteRepoStatusClient};
}
