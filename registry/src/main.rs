mod client;
mod codec;
mod ipc;
mod registry;
mod server;

use actix::prelude::*;

use failure::Error;

use ipc::{Printer, ServerRouter};

use registry::RegistryRequest;

use tokio::net::UnixStream;

use client::{InterfaceRequest, SystemResponder, WriteInterface};

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let path = "/tmp/central.registry";

    //ServerRouter::serve(path).context(format!("Error serving on ipc path: {}", path))?;
    run_as_client(path).await?;

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}

async fn run_as_client(path: &str) -> Result<(), Error> {
    let path = path.to_string();
    let stream = UnixStream::connect(path).await?;
    let (r, w) = tokio::io::split(stream);

    Printer::subscribe(r)?;
    let addr = WriteInterface::<RegistryRequest>::attach(w).await?;

    addr.send(InterfaceRequest(RegistryRequest::List)).await?;

    Ok(())
}
