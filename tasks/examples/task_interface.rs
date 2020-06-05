use std::thread::sleep;
use std::time::Duration;

use actix::prelude::*;

use failure::Error;

use lipsum::lipsum_title;

use registry::{InterfaceClient, Require};
use tasks::{CommandClient, Complete, Create, Get, GetOne, QueryClient, TaskQuery};

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let client = InterfaceClient::connect_default().await?;

    let cmd_path = client
        .send(Require {
            capability: "task_command_provider".to_string(),
        })
        .await?;

    pm(cmd_path.clone()).await?;

    let qry_path = client
        .send(Require {
            capability: "task_query_provider".to_string(),
        })
        .await?;

    worker(cmd_path, qry_path.clone()).await?;

    board(qry_path).await?;

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}

async fn pm(path: String) -> Result<(), Error> {
    Arbiter::new().send(Box::pin(async move {
        let client = CommandClient::connect(path.as_str()).await.unwrap();
        // Create client
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

async fn worker(command: String, query: String) -> Result<(), Error> {
    Arbiter::new().send(Box::pin(async move {
        let cmd_client = CommandClient::connect(command.as_str()).await.unwrap();
        let qry_client = QueryClient::connect(query.as_str()).await.unwrap();
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

async fn board(path: String) -> Result<(), Error> {
    Arbiter::new().send(Box::pin(async move {
        let client = QueryClient::connect(path.as_str()).await.unwrap();
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
