use std::thread::sleep;
use std::time::Duration;

use actix::prelude::*;

use failure::Error;

use lipsum::lipsum_title;

use registry::interface;

use tasks::{Complete, Create, Get, GetOne, TaskCommandClient, TaskQuery, TaskQueryClient};

interface! {
    TaskCommand,
    TaskQuery
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    pm().await?;

    worker().await?;

    board().await?;

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}

async fn pm() -> Result<(), Error> {
    Arbiter::new().send(Box::pin(async move {
        let client = require::<TaskCommandClient>().await.unwrap();

        loop {
            if let Err(e) = client
                .send(Create {
                    name: lipsum_title(),
                })
                .await
            {
                println!("PM: Error {}", e);
            };

            sleep(Duration::from_secs(3));
        }
    }));

    Ok(())
}

async fn worker() -> Result<(), Error> {
    Arbiter::new().send(Box::pin(async move {
        let cmd_client = require::<TaskCommandClient>().await.unwrap();
        let qry_client = require::<TaskQueryClient>().await.unwrap();
        // Create client
        loop {
            match qry_client
                .send(GetOne {
                    query: TaskQuery::Done(false),
                })
                .await
            {
                Ok(Some(task)) => {
                    println!("Worker: Completing task: {}", task.name);
                    cmd_client
                        .send(Complete { task_id: task.id })
                        .await
                        .unwrap();
                }
                Err(e) => println!("Worker: Mailbox Error: {}", e),
                _ => println!("Worker: Nothing to do"),
            }

            sleep(Duration::from_secs(5));
        }
    }));

    Ok(())
}

async fn board() -> Result<(), Error> {
    Arbiter::new().send(Box::pin(async move {
        let client = require::<TaskQueryClient>().await.unwrap();
        // Create client
        loop {
            match client
                .send(Get {
                    query: TaskQuery::Done(false),
                })
                .await
            {
                Ok(tasks) => {
                    println!("==> ToDo:");
                    for task in tasks {
                        println!("====> {}", task.name);
                    }
                }
                Err(e) => println!("Worker: Mailbox Error: {}", e),
            }

            sleep(Duration::from_secs(2));
        }
    }));

    Ok(())
}
