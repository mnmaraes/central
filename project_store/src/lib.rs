#[macro_use]
extern crate diesel;

mod ipc;
mod models;

pub mod model {
    pub use crate::models::Project;
}

pub mod command_client {
    pub use crate::ipc::{Create, Delete, ProjectCommandClient};
}

pub mod query_client {
    pub use crate::ipc::{Get, ProjectQueryClient};
}

pub mod status_client {
    pub use crate::ipc::{Check, ProjectStoreStatusClient};
}
