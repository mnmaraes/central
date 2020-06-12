use std::collections::HashMap;
use std::env::var;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

use dialoguer::Select;

use tempfile::Builder;

use tokio::time::timeout;

use rayon::prelude::*;

use note_store::command_client::{Create, Delete, NoteCommandClient, Update};
use note_store::model::Note;
use note_store::query_client::{Get, NoteQueryClient};
use note_store::status_client::{Check, NoteStoreStatusClient};

use registry::{interface, StatusClient};

interface! {
    NoteCommand,
    NoteQuery,
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
    let file = Builder::new()
        .suffix(".md")
        .tempfile()
        .expect("Couldn't create a temporary file");

    let editor_cmd = match var("EDITOR") {
        Ok(editor) => editor,
        Err(_) => "vi".to_string(),
    };

    actix_rt::System::new("main").block_on(async move {
        let status_client = require::<NoteStoreStatusClient>().await.unwrap();
        status_client
            .send(Check)
            .await
            .expect("Couldn't contact Note Store. Make sure it is running and registered");

        Command::new(editor_cmd)
            .arg(file.path().to_str().unwrap())
            .status()
            .expect("Failed to start editor");

        let mut contents = String::new();
        BufReader::new(file.into_file())
            .read_to_string(&mut contents)
            .expect("Couldn't read file");

        let note_client = require::<NoteCommandClient>().await.unwrap();
        note_client
            .send(Create { body: contents })
            .await
            .expect("Failed To Notify Note Sore");
    });
}

pub fn delete_note() {
    actix_rt::System::new("main").block_on(async move {
        let query_client = require::<NoteQueryClient>().await.unwrap();
        let notes: Vec<Note> = query_client
            .send(Get)
            .await
            .expect("Couldn't fetch existing notes")
            .expect("Couldn't fetch existing notes");

        let first_lines: Vec<_> = notes
            .iter()
            .filter_map(|note| note.body.lines().next())
            .collect();

        let selection = Select::new()
            .with_prompt("Select note to delete:")
            .items(&first_lines)
            .interact()
            .unwrap();

        let note_client = require::<NoteCommandClient>().await.unwrap();
        note_client
            .send(Delete {
                id: notes[selection].id.to_string(),
            })
            .await
            .expect("Failed To Notify Note Sore");
    });
}

pub fn update_note() {
    let mut file = Builder::new()
        .suffix(".md")
        .tempfile()
        .expect("Couldn't create a temporary file");

    let editor_cmd = match var("EDITOR") {
        Ok(editor) => editor,
        Err(_) => "vi".to_string(),
    };

    actix_rt::System::new("main").block_on(async move {
        let query_client = require::<NoteQueryClient>().await.unwrap();
        let notes: Vec<Note> = query_client
            .send(Get)
            .await
            .expect("Couldn't fetch existing notes")
            .expect("Couldn't fetch existing notes");

        let first_lines: Vec<_> = notes
            .iter()
            .filter_map(|note| note.body.lines().next())
            .collect();

        let selection = Select::new()
            .with_prompt("Select note to update:")
            .items(&first_lines)
            .interact()
            .unwrap();

        BufWriter::new(file.as_file_mut())
            .write_all(notes[selection].body.as_bytes())
            .expect("Couldn't write note to file");

        Command::new(editor_cmd)
            .arg(file.path().to_str().unwrap())
            .status()
            .expect("Failed to start editor");

        let mut contents = String::new();
        BufReader::new(file.reopen().unwrap())
            .read_to_string(&mut contents)
            .expect("Couldn't read file");

        let note_client = require::<NoteCommandClient>().await.unwrap();
        note_client
            .send(Update {
                id: notes[selection].id.to_string(),
                body: contents,
            })
            .await
            .expect("Failed To Notify Note Sore");
    });
}
