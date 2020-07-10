use std::env::{current_dir, set_current_dir, var};
use std::io::BufRead;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use actix::prelude::*;

use tracing::{error, info};

use super::index::{Deindex, Index, NoteIndex, Reindex};
use super::types::ContextItem;

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

pub fn parse_path(path: &str) -> Vec<ContextItem> {
    path.replace(".md", "")
        .split('/')
        .map(|part| ContextItem::Simple { name: part.into() })
        .collect()
}

fn is_md(path: &PathBuf) -> bool {
    path.extension().is_some() && path.extension().unwrap() == "md"
}
