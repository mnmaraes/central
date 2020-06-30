mod utils;

use std::collections::HashMap;
use std::io::BufRead;
use std::process::Command;
use std::time::{Duration, Instant};

use tokio::time::timeout;

use rayon::prelude::*;

use note_store::command_client::{Create, Delete, NoteCommandClient, Update};
use note_store::model::NoteRef;
use note_store::status_client::{Check, NoteStoreStatusClient};

use utils::*;

use registry::{interface, StatusClient};

interface! {
    NoteCommand,
    NoteStoreStatus
}

pub fn check_status() {
    let ps = Command::new("ps")
        .arg("aux")
        .output()
        .expect("Couldn't list processes");

    let lines: Vec<_> = ps.stdout.lines().collect();
    let processes: HashMap<String, String> = lines
        .par_iter()
        .filter_map(|res| match res {
            Ok(line)
                if ["registry", "note_store"]
                    .iter()
                    .any(|pattern| line.contains(*pattern)) =>
            {
                let info = line.par_split_whitespace().collect::<Vec<&str>>();
                Some((info[10].to_string(), info[1].to_string()))
            }
            _ => None,
        })
        .collect();

    actix_rt::System::new("main").block_on(async move {
        println!("service pid μs status");
        if let Some(pid) = processes.get("registry") {
            let start = Instant::now();
            match timeout(Duration::from_secs(1), StatusClient::check_default()).await {
                Ok(_) => println!("Registry: {} {} Ok", pid, start.elapsed().as_micros()),
                Err(e) => println!(
                    "Registry: {} {} Error({})",
                    pid,
                    start.elapsed().as_micros(),
                    e
                ),
            }
        }
        if let Some(pid) = processes.get("note_store") {
            let error_str = format!("Couldn't connect to NoteStore({})", pid);
            let client = require::<NoteStoreStatusClient>().await.expect(&error_str);
            let start = Instant::now();
            match timeout(Duration::from_secs(1), client.send(Check)).await {
                Ok(_) => println!("NoteStore: {} {} Ok", pid, start.elapsed().as_micros()),
                Err(e) => println!(
                    "NoteStore: {} {} Error({})",
                    pid,
                    start.elapsed().as_micros(),
                    e
                ),
            }
        }
    });
}

pub fn create_note() {
    let editor = TmpEditor::new();

    actix_rt::System::new("main").block_on(async move {
        let status_client = require::<NoteStoreStatusClient>().await.unwrap();
        status_client
            .send(Check)
            .await
            .expect("Couldn't contact Note Store. Make sure it is running and registered");

        editor.open().expect("Editor exited with error code");

        let mut contents = String::new();

        editor
            .read_contents_to_string(&mut contents)
            .expect("Couldn't read file");

        let note_client = require::<NoteCommandClient>().await.unwrap();
        note_client
            .send(Create {
                reference: NoteRef::Deferred,
                body: contents,
            })
            .await
            .expect("Failed To Notify Note Sore");
    });
}

pub fn list_notes() {
    actix_rt::System::new("main").block_on(async move {
        get_notes()
            .await
            .expect("Couldn't fetch notes")
            .iter()
            .map(|note| note.title.clone())
            .for_each(|line| println!("{}", line));
    });
}

pub fn delete_note() {
    actix_rt::System::new("main").block_on(async move {
        let (reference, _) = select_note().await.expect("Couldn't select note");

        let note_client = require::<NoteCommandClient>().await.unwrap();
        note_client
            .send(Delete { reference })
            .await
            .expect("Failed To Notify Note Sore");
    });
}

pub fn update_note() {
    let mut editor = TmpEditor::new();

    actix_rt::System::new("main").block_on(async move {
        let (reference, body) = select_note().await.expect("Couldn't select note");

        editor
            .load_contents(&body)
            .expect("Couldn't write note to file");

        editor.open().expect("Failed to start editor");

        let mut contents = String::new();
        editor
            .read_contents_to_string(&mut contents)
            .expect("Couldn't read file");

        let note_client = require::<NoteCommandClient>().await.unwrap();
        note_client
            .send(Update {
                reference,
                body: contents,
            })
            .await
            .expect("Failed To Notify Note Sore");
    });
}
