mod ipc;
mod runners;

use ipc::{NoteCommandRequest, NoteQueryRequest, NoteRepo, NoteRepoStatusRequest};

registry::run_provide! {
    NoteRepo => [NoteCommand, NoteQuery, NoteRepoStatus]
}
