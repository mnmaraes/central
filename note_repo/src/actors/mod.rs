use std::collections::HashSet;
use std::env::{current_dir, set_current_dir, var};
use std::fs::read_to_string;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use core::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use actix::prelude::*;

use tracing::{error, info};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Parse(PathBuf);

#[derive(Debug, Clone)]
enum ContextItem {
    Simple {
        name: String,
    },
    Heading {
        level: u32,
        start: u32,
        title: String,
    },
}

#[derive(Debug, Clone)]
enum ParseState {
    Heading {
        level: u32,
        start: usize,
        content: String,
    },
    ListItem {
        is_done: Option<bool>,
        start: usize,
        content: String,
    },
    Paragraph {
        start: usize,
        content: String,
    },
    Link {
        start: usize,
        destination: String,
        content: String,
    },
}

impl ParseState {
    fn set_task_value(&self, is_done: bool) -> ParseState {
        let clone = self.clone();
        if let ParseState::ListItem { start, content, .. } = clone {
            ParseState::ListItem {
                is_done: Some(is_done),
                start,
                content,
            }
        } else {
            clone
        }
    }

    fn append_text(&self, text: &str) -> ParseState {
        use ParseState::*;
        match self.clone() {
            Heading {
                level,
                start,
                content,
            } => Heading {
                level,
                start,
                content: format!("{}{}", content, text),
            },
            ListItem {
                is_done,
                start,
                content,
            } => ListItem {
                is_done,
                start,
                content: format!("{}{}", content, text),
            },
            Paragraph { start, content } => Paragraph {
                start,
                content: format!("{}{}", content, text),
            },
            Link {
                start,
                destination,
                content,
            } => Link {
                start,
                destination,
                content: format!("{}{}", content, text),
            },
        }
    }
}

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
            Event::Start(Tag::Item) => self.handle_state(ParseState::ListItem {
                is_done: None,
                start: range.start,
                content: "".into(),
            }),
            Event::Start(Tag::Paragraph) => self.handle_state(ParseState::Paragraph {
                start: range.start,
                content: "".into(),
            }),
            Event::Start(Tag::Link(_, uri, _)) => self.handle_state(ParseState::Link {
                start: range.start,
                destination: uri.to_string(),
                content: "".into(),
            }),
            Event::TaskListMarker(is_done) => self.set_current_item_task(is_done),
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
            None => 0,
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

        self.index.do_send(IndexContent::Link {
            start,
            destination,
            content,
            context: self.context.clone(),
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

        self.index.do_send(IndexContent::Paragraph {
            start,
            content,
            context: self.context.clone(),
        })
    }

    fn index_current_list_item(&mut self) {
        let item = self.state.pop();
        let (is_done, start, name) = match item {
            Some(ParseState::ListItem {
                is_done,
                start,
                content,
            }) => (is_done, start as _, content),
            _ => {
                error!("Expected a task, found: {:?}", item);
                panic!("Unhandled state");
            }
        };

        if let Some(is_done) = is_done {
            self.index.do_send(IndexContent::Task {
                is_done,
                start,
                name,
                context: self.context.clone(),
            })
        }
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

        self.index.do_send(IndexContent::Heading {
            start,
            level,
            context: self.context.clone(),
        })
    }

    fn set_current_item_task(&mut self, is_done: bool) {
        info!("Setting task value {}", is_done);
        for state in self.state.iter_mut() {
            *state = state.set_task_value(is_done);
        }
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
        self.context = path
            .replace(".md", "")
            .split('/')
            .map(|part| ContextItem::Simple { name: part.into() })
            .collect();
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

#[derive(Message)]
#[rtype(result = "()")]
struct Index(PathBuf);

#[derive(Message)]
#[rtype(result = "()")]
struct Reindex(PathBuf);

#[derive(Message)]
#[rtype(result = "()")]
struct Deindex(PathBuf);

#[derive(Message, Debug)]
#[rtype(result = "()")]
enum IndexContent {
    Heading {
        context: Vec<ContextItem>,
        level: u32,
        start: u32,
    },
    Task {
        context: Vec<ContextItem>,
        is_done: bool,
        start: u32,
        name: String,
    },
    Paragraph {
        context: Vec<ContextItem>,
        start: u32,
        content: String,
    },
    Link {
        context: Vec<ContextItem>,
        start: u32,
        content: String,
        destination: String,
    },
}

#[derive(Debug)]
pub struct NoteIndex {
    references: HashSet<PathBuf>,
    parser: Addr<NoteParser>,
}

impl Actor for NoteIndex {
    type Context = Context<Self>;
}

impl NoteIndex {
    pub fn new(parser: Addr<NoteParser>) -> Self {
        NoteIndex {
            references: HashSet::new(),
            parser,
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

impl Handler<Index> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Index, ctx: &mut Self::Context) -> Self::Result {
        info!("Indexing {:?}", msg.0);
        self.parse(&msg.0);
        // TODO: Any note processing goes here
        self.references.insert(msg.0);
    }
}

impl Handler<Reindex> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Reindex, _ctx: &mut Self::Context) -> Self::Result {
        info!("Reindexing {:?}", msg.0);
        self.parse(&msg.0);
        //TODO: Note Reprocessing go here
        if !self.references.contains(&msg.0) {
            info!("Reindexed note {:?} wasn't present. Inserting now", msg.0);
            self.references.insert(msg.0);
        }
    }
}

impl Handler<Deindex> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Deindex, _ctx: &mut Self::Context) -> Self::Result {
        info!("Deindexing {:?}", msg.0);
        //TODO: Any action needed after a note removal goes here
        self.references.remove(&msg.0);
    }
}

impl Handler<IndexContent> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: IndexContent, _ctx: &mut Self::Context) -> Self::Result {
        info!("Indexing Content {:?}", msg);
        //TODO
    }
}

fn get_file_paths() -> Vec<String> {
    info!("Working from: {:?}", current_dir());
    let ls = Command::new("fd")
        .args(&["-e", "md", "-c", "never"])
        .output()
        .unwrap();

    ls.stdout.lines().filter_map(|res| res.ok()).collect()
}

pub fn set_home() {
    dotenv::dotenv().ok();

    let current = current_dir();
    let home = var("NOTE_HOME").map(|s| s.into()).or(current).unwrap();

    set_current_dir(&home).ok();
}

pub fn start_watch(index: &Addr<NoteIndex>) {
    Arbiter::new().send(Box::pin({
        let index = index.clone();
        async move {
            let (tx, rx) = std::sync::mpsc::channel();

            let mut watcher: RecommendedWatcher = watcher(tx, Duration::from_secs(1)).unwrap();

            watcher.watch(".", RecursiveMode::Recursive).unwrap();

            loop {
                match rx.recv() {
                    Ok(event) => {
                        info!("Received Watcher Event {:?}", event);
                        process_event(event, &index);
                    }
                    Err(e) => info!("Watcher Error: {:?}", e),
                }
            }
        }
    }));

    for path in get_file_paths().iter().cloned() {
        Arbiter::new().send(Box::pin({
            let index = index.clone();
            async move {
                if let Err(e) = index.send(Index(path.clone().into())).await {
                    error!("Error: {:?}  \nIndexing: {:?}", e, path)
                };
            }
        }))
    }
}

fn process_event(event: DebouncedEvent, index: &Addr<NoteIndex>) {
    match event {
        DebouncedEvent::Write(path) if is_md(&path) => index.do_send(Reindex(path)),
        DebouncedEvent::Create(path) if is_md(&path) => index.do_send(Index(path)),
        DebouncedEvent::Remove(path) if is_md(&path) => index.do_send(Deindex(path)),
        DebouncedEvent::Rename(from, to) => {
            if is_md(&from) {
                index.do_send(Deindex(from))
            }
            if is_md(&to) {
                index.do_send(Index(to))
            }
        }
        _ => { /* Do Nothing */ }
    }
}

fn is_md(path: &PathBuf) -> bool {
    path.extension().is_some() && path.extension().unwrap() == "md"
}
