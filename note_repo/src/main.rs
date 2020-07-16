mod actors;
mod ipc;
mod runners;

use actors::{set_home, start_watch, NoteIndex, NoteParser};
use ipc::{
    NoteCommandRequest, NoteIndexRequest, NoteQueryRequest, NoteRepo, NoteRepoStatusRequest,
};

registry::run_provide! {
    NoteRepo {
        setup => {
            set_home();
            let index = NoteIndex::create(|ctx| {
                let address = ctx.address();
                let parser = SyncArbiter::start(4, move || NoteParser::new(address.clone()));

                NoteIndex::new(parser)
            });
            start_watch(&index);

        },
        provider => NoteRepo::new(index).start()
    } => [NoteCommand, NoteQuery, NoteRepoStatus, NoteIndex]
}
