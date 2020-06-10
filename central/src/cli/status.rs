use std::collections::HashMap;
use std::io::BufRead;
use std::process::Command;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use tokio::time::timeout;

use clap::Clap;

use note_store::{Check, NoteStoreStatusClient};
use registry::{interface, StatusClient};

interface! {
    NoteStoreStatus
}

/// Central services status checks
#[derive(Clap)]
pub struct Status;

impl Status {
    pub fn run(&self) {
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
}
