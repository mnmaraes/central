use actix::prelude::*;

use failure::Error;

use uuid::Uuid;

use registry::{ProviderClient, Register};
use tasks::{TaskStore, serve_command, serve_query};

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let store = TaskStore::start_default();

    // register Command Provider
    let path = format!("/tmp/central.task_command_provider.{}", Uuid::new_v4());

    serve_command(path.as_str(), &store)?;
    let client = ProviderClient::register_default("task_command_provider", path.as_str()).await?;

    println!("Awaiting Commands");

    // register Query Provider
    let path = format!("/tmp/central.task_query_provider.{}", Uuid::new_v4());

    serve_query(path.as_str(), &store)?;
    client
        .send(Register {
            capability: "task_query_provider".to_string(),
            address: path,
        })
        .await?;

    println!("Awaiting Queries");
    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}
