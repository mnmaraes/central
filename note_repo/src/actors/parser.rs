use std::fs::read_to_string;
use std::path::PathBuf;

use core::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

use actix::prelude::*;

use tracing::{error, info};

use super::index::{IndexContent, NoteIndex};
use super::types::{ContextItem, IndexedContent, ParseState};
use super::utils::parse_path;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Parse(pub PathBuf);

#[derive(Debug)]
pub struct NoteParser {
    index: Addr<NoteIndex>,

    context: Vec<ContextItem>,
    state: Vec<ParseState>,
}

impl NoteParser {
    pub fn new(index: Addr<NoteIndex>) -> Self {
        Self {
            index,

            context: vec![],
            state: vec![],
        }
    }
}

impl NoteParser {
    fn handle_event<'a>(&mut self, event: Event<'a>, range: Range<usize>) {
        //TODO: Handle it!
        match event {
            Event::Start(Tag::Heading(level)) => {
                self.handle_state(ParseState::Heading {
                    level,
                    start: range.start,
                    content: "".into(),
                });
            }
            Event::Start(Tag::Paragraph) => self.handle_state(ParseState::Paragraph {
                start: range.start,
                content: "".into(),
            }),
            Event::Start(Tag::Link(_, uri, _)) => self.handle_state(ParseState::Link {
                start: range.start,
                destination: uri.to_string(),
                content: "".into(),
            }),
            Event::TaskListMarker(is_done) => self.handle_state(ParseState::ListItem {
                is_done,
                start: range.start,
                content: "".into(),
            }),
            Event::Text(txt) => self.append_text(txt.to_string()),
            Event::SoftBreak => self.append_text("\n".into()),
            Event::End(Tag::Heading(_)) => self.index_current_header(),
            Event::End(Tag::Item) => self.index_current_list_item(),
            Event::End(Tag::Paragraph) => self.index_current_paragraph(),
            Event::End(Tag::Link(_, _, _)) => self.index_current_link(),
            // Don't know/care what this is is: DO NOTHING
            _ => {}
        }
    }

    fn current_context_level(&self) -> u32 {
        let top = self.context.iter().last();
        match top {
            Some(ContextItem::Heading { level, .. }) => *level,
            Some(ContextItem::Simple { .. }) => 0,
            _ => 0,
        }
    }

    fn index_current_link(&mut self) {
        let item = self.state.pop();
        let (start, content, destination) = match item {
            Some(ParseState::Link {
                start,
                content,
                destination,
            }) => (start as _, content, destination),
            _ => {
                error!("Expected a link, found: {:?}", item);
                panic!("Unhandled state");
            }
        };

        self.index.do_send(IndexContent {
            context: self.context.clone(),
            content: IndexedContent::Link {
                start,
                content,
                destination,
            },
        })
    }

    fn index_current_paragraph(&mut self) {
        let item = self.state.pop();
        let (start, content) = match item {
            Some(ParseState::Paragraph { start, content }) => (start as _, content),
            _ => {
                error!("Expected a paragraph, found: {:?}", item);
                panic!("Unhandled state");
            }
        };

        self.index.do_send(IndexContent {
            context: self.context.clone(),
            content: IndexedContent::Paragraph { start, content },
        })
    }

    fn index_current_list_item(&mut self) {
        let last = self.state.last();
        let (is_done, start, name) = match last {
            Some(ParseState::ListItem {
                is_done,
                start,
                content,
            }) => (*is_done, *start as _, content.clone()),
            _ => {
                return;
            }
        };

        self.state.pop();
        self.index.do_send(IndexContent {
            context: self.context.clone(),
            content: IndexedContent::Task {
                is_done,
                start,
                name,
            },
        })
    }

    fn index_current_header(&mut self) {
        let item = self.state.pop();
        let (level, start, title) = match item {
            Some(ParseState::Heading {
                level,
                start,
                content,
            }) => (level, start as _, content),
            _ => {
                error!("Expected a header, found: {:?}", item);
                panic!("Unhandled state");
            }
        };

        self.context.push(ContextItem::Heading {
            level,
            start,
            title,
        });

        // NOTE: Index Headers, maybe??
    }

    fn append_text(&mut self, text: String) {
        for state in self.state.iter_mut() {
            *state = state.append_text(&text);
        }
    }

    fn handle_state(&mut self, state: ParseState) {
        if let ParseState::Heading { level, .. } = state {
            self.pop_context_to_level(level)
        }
        self.state.push(state);
    }

    fn pop_context_to_level(&mut self, level: u32) {
        while self.current_context_level() >= level {
            self.context.pop();
        }
    }

    fn set_context(&mut self, path: &str) {
        self.context = parse_path(path);
    }
}

impl Actor for NoteParser {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Parser Started");
    }
}

impl Handler<Parse> for NoteParser {
    type Result = ();

    fn handle(&mut self, msg: Parse, _ctx: &mut Self::Context) -> Self::Result {
        let path = msg.0;
        info!("Received Parsing Request: {:?}", path);

        info!("Parsing: {:?}", path);
        let default = String::new();
        let content = read_to_string(&path).unwrap_or(default);
        let options = Options::all();
        let parser = Parser::new_ext(&content, options);

        self.set_context(path.to_str().unwrap_or(""));
        for (event, range) in parser.into_offset_iter() {
            self.handle_event(event, range);
        }
        info!("Done parsing {:?}", path);
    }
}
