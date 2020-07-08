use std::collections::{HashMap, HashSet};
use std::env::{current_dir, set_current_dir, var};
use std::fs::read_to_string;
use std::hash::Hash;
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

#[derive(Debug, Hash, PartialEq, Eq)]
enum IndexedContent {
    Paragraph {
        start: usize,
        content: String,
    },
    Task {
        start: usize,
        is_done: bool,
        name: String,
    },
    Link {
        start: u32,
        content: String,
        destination: String,
    },
}

#[derive(Debug)]
struct ContextTree {
    children: HashMap<ContextItem, ContextTree>,
    content: HashSet<IndexedContent>,
}

impl ContextTree {
    pub fn new() -> ContextTree {
        ContextTree {
            children: HashMap::new(),
            content: HashSet::new(),
        }
    }

    pub fn get(&self, path: &[ContextItem]) -> Option<&ContextTree> {
        let mut entry = self;
        for segment in path {
            entry = match entry.children.get(segment) {
                Some(entry) => entry,
                None => return None,
            }
        }

        Some(entry)
    }

    pub fn get_mut(&mut self, path: &[ContextItem]) -> Option<&mut ContextTree> {
        let mut entry = self;
        for segment in path {
            entry = match entry.children.get_mut(segment) {
                Some(entry) => entry,
                None => return None,
            }
        }
        Some(entry)
    }

    pub fn index(&mut self, path: &[ContextItem], content: IndexedContent) {
        self.extend(path);
        self.get_mut(path).unwrap().content.insert(content);
    }

    pub fn deindex(&mut self, path: &[ContextItem]) {
        let mut path = path.to_vec();
        let key = path.pop().unwrap();
        if let Some(tree) = self.get_mut(&path) {
            tree.children.remove(&key);
        }
    }

    fn extend(&mut self, path: &[ContextItem]) {
        if path.is_empty() {
            return;
        }
        let mut path = path.to_vec();

        let child = path.remove(0);
        let sub_tree = self.children.entry(child).or_insert_with(ContextTree::new);
        sub_tree.extend(&path);
    }
}

impl std::fmt::Display for ContextTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_self("", f)?;

        Ok(())
    }
}

impl ContextTree {
    fn fmt_self(&self, prepend: &str, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prepend = format!("{}>", prepend);
        for (k, v) in self.children.iter() {
            writeln!(f, "{} {}", prepend, k)?;
            v.fmt_self(&prepend, f)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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

impl std::fmt::Display for ContextItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ContextItem::*;

        match self {
            Simple { name } => write!(f, "{}", name)?,
            Heading { title, .. } => write!(f, "{}", title)?,
        }

        Ok(())
    }
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
            self.index.do_send(IndexContent {
                context: self.context.clone(),
                content: IndexedContent::Task {
                    is_done,
                    start,
                    name,
                },
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

        // NOTE: Index Headers, maybe??
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
struct IndexContent {
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

impl Handler<Index> for NoteIndex {
    type Result = ();

    fn handle(&mut self, msg: Index, ctx: &mut Self::Context) -> Self::Result {
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

fn parse_path(path: &str) -> Vec<ContextItem> {
    path.replace(".md", "")
        .split('/')
        .map(|part| ContextItem::Simple { name: part.into() })
        .collect()
}

fn is_md(path: &PathBuf) -> bool {
    path.extension().is_some() && path.extension().unwrap() == "md"
}
