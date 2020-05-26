mod ipc;
mod registry;

use actix::prelude::*;

use failure::{Error, ResultExt};

use crate::registry::Registry;

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let path = "/tmp/central.registry";

    Registry::serve(path).context(format!("Error serving on ipc path: {}", path))?;

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}
