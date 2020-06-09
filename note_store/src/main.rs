#[macro_use]
extern crate diesel;

mod ipc;
mod models;
mod schema;

use registry::run_provide;

use ipc::{NoteCommandRequest, NoteQueryRequest, NoteStore};

run_provide! {
    NoteStore => [NoteCommand, NoteQuery]
}
