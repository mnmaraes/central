use std::path::PathBuf;

use actix::prelude::*;

use tracing::{error, info};

use super::parser::{NoteParser, Parse};
use super::types::{ContextItem, ContextTree, IndexedContent};
use super::utils::parse_path;

#[derive(Message, Debug)]
#[rtype(result = "Vec<String>")]
pub struct ListTasks;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Index(pub PathBuf);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Reindex(pub PathBuf);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Deindex(pub PathBuf);

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct IndexContent {
    pub context: Vec<ContextItem>,
    pub content: IndexedContent,
}

#[derive(Debug)]
pub struct NoteIndex {
    parser: Addr<NoteParser>,
    context: ContextTree,
}

impl Actor for NoteIndex {
    type Context = Context<Self>;
}

impl NoteIndex {
    pub fn new(parser: Addr<NoteParser>) -> Self {
        NoteIndex {
            parser,
            context: ContextTree::new(),
        }
    }

    fn parse(&self, path: &PathBuf) {
        let parser = self.parser.clone();
        let path = path.clone();
        Arbiter::current().send(Box::pin(async move {
            if let Err(e) = parser.send(Parse(path.clone())).await {
                error!("Error parsing {:?}: {:?}", path, e);
            };
        }));
    }
}

impl Handler<ListTasks> for NoteIndex {
    type Result = MessageResult<ListTasks>;

    fn handle(&mut self, msg: ListTasks, _ctx: &mut Self::Context) -> Self::Result {
        info!("Empty {:?}", msg);
        MessageResult(
            self.context
                .indexed_content(&[])
                .iter()
                .filter_map(|c| {
                    if let IndexedContent::Task { name, is_done, .. } = c {
                        if *is_done {
                            None
                        } else {
                            Some(name.clone())
                        }
                    } else {
                        None
                    }
                })
                .collect(),
        )
        // TODO: Any note processing goes here (Just parsing for now)
    }
}

impl Handler<Index> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Index, _ctx: &mut Self::Context) -> Self::Result {
        info!("Indexing {:?}", msg.0);
        self.parse(&msg.0);
        // TODO: Any note processing goes here (Just parsing for now)
    }
}

impl Handler<Reindex> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Reindex, _ctx: &mut Self::Context) -> Self::Result {
        info!("Reindexing {:?}", msg.0);
        self.context
            .deindex(&parse_path(msg.0.to_str().unwrap_or_default()));
        self.parse(&msg.0);
        //TODO: Note Reprocessing go here (may need to remove indexed data under the context)
    }
}

impl Handler<Deindex> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Deindex, _ctx: &mut Self::Context) -> Self::Result {
        info!("Deindexing {:?}", msg.0);
        self.context
            .deindex(&parse_path(msg.0.to_str().unwrap_or_default()));
        //TODO: Any action needed after a note removal goes here
    }
}

impl Handler<IndexContent> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: IndexContent, _ctx: &mut Self::Context) -> Self::Result {
        let IndexContent {
            context: path,
            content,
        } = msg;

        self.context.index(&path, content);
        //TODO: Index content for search(?)
    }
}
