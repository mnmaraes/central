mod codec;
mod registry;
mod server;

use std::{fs, io::ErrorKind};

use actix::prelude::*;

use futures_util::future::FutureExt;

use tokio::net::{UnixListener, UnixStream};
use tokio::stream::StreamExt;

use failure::{Error, ResultExt};

use registry::Registry;
use server::{ClientRequest, IpcClient, IpcConnect, IpcServer};

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let path = "/tmp/central.registry";

    run_as_server(path).await?;
    //run_as_client(path.to_string()).await?;

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}

async fn run_as_client(path: String) -> Result<(), Error> {
    Arbiter::spawn(UnixStream::connect(path).then(|stream| {
        let stream = stream.unwrap();
        let addr = IpcClient::create(|ctx| IpcClient::new(stream, ctx));

        addr.do_send(ClientRequest::Register("Linker".to_string()));

        async {}
    }));

    Ok(())
}

async fn run_as_server(path: &str) -> Result<(), Error> {
    let registry = Registry::start_default();

    let listener = Box::new(open_uds_listener(path).context("Couldn't open socket")?);

    IpcServer::create(move |ctx| {
        ctx.add_message_stream(Box::leak(listener).incoming().map(|stream| {
            let stream = stream.unwrap();
            let addr = stream.peer_addr().unwrap();
            IpcConnect(stream, addr)
        }));
        IpcServer::new(registry)
    });

    Ok(())
}

fn open_uds_listener(path: &str) -> Result<UnixListener, Error> {
    match UnixListener::bind(&path) {
        Ok(l) => Ok(l),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            // 1. Handle cases where file exists
            // TODO: Handle it more gracefully (Ask user whether to force or abort)
            println!("A connection file already exists. Removing it.");
            fs::remove_file(&path)?;

            UnixListener::bind(&path).map_err(Error::from)
        }
        Err(e) => Err(Error::from(e)),
    }
}
