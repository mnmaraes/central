#[macro_use]
extern crate diesel;

mod ipc;
mod models;
mod schema;

use registry::run_provide;

use ipc::{NoteCommandRequest, NoteStore, StatusRequest};

run_provide! {
    NoteStore => [NoteCommand, Status]
}
