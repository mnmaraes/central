use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub enum ParseState {
    Heading {
        level: u32,
        start: usize,
        content: String,
    },
    ListItem {
        is_done: bool,
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
    pub fn append_text(&self, text: &str) -> ParseState {
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

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum IndexedContent {
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
pub struct ContextTree {
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

    pub fn indexed_content(&self, path: &[ContextItem]) -> Vec<IndexedContent> {
        if let Some(tree) = self.get(path) {
            tree.all_content()
        } else {
            vec![]
        }
    }

    fn all_content(&self) -> Vec<IndexedContent> {
        let mut content: Vec<_> = self.content.iter().cloned().collect();

        for child in self.children.values() {
            content.extend(child.all_content());
        }

        content
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
pub enum ContextItem {
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
