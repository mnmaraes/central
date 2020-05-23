mod codec;
mod registry;
mod server;

use std::{fs, io::ErrorKind};

use actix::prelude::*;

use futures_util::future::FutureExt;

use tokio::net::{UnixListener, UnixStream};
use tokio::stream::StreamExt;

use failure::{Error, ResultExt};

use registry::{ListCapabilities, Register, Registry, RegistryRequest, RegistryResponse};
use server::{IpcServer, Router};

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

//async fn run_as_client(path: String) -> Result<(), Error> {
//Arbiter::spawn(UnixStream::connect(path).then(|stream| {
//let stream = stream.unwrap();
//let addr = IpcClient::create(|ctx| IpcClient::new(stream, ctx));

//addr.do_send(ClientRequest::Register("Linker".to_string()));

//async {}
//}));

//Ok(())
//}

struct ServerRouter {
    registry: Addr<Registry>,
}

impl Actor for ServerRouter {
    type Context = Context<Self>;
}

impl Handler<RegistryRequest> for ServerRouter {
    type Result = ResponseActFuture<Self, RegistryResponse>;

    fn handle(&mut self, msg: RegistryRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RegistryRequest::List => {
                Box::pin(self.registry.send(ListCapabilities).into_actor(self).map(
                    |res, _act, _ctx| match res {
                        Ok(capabilities) => RegistryResponse::Capabilities(capabilities),
                        Err(e) => {
                            RegistryResponse::Error(format!("Error Listing Capabilities: {:?}", e))
                        }
                    },
                ))
            }
            RegistryRequest::Register(capability) => Box::pin(
                self.registry
                    .send(Register::new(capability))
                    .into_actor(self)
                    .map(|res, _act, _ctx| match res {
                        Ok(_) => RegistryResponse::Registered,
                        Err(e) => {
                            RegistryResponse::Error(format!("Error Listing Capabilities: {:?}", e))
                        }
                    }),
            ),
        }
    }
}

impl Router<RegistryRequest> for ServerRouter {}

impl ServerRouter {
    fn route() -> Addr<Self> {
        let registry = Registry::start_default();

        Self { registry }.start()
    }
}

async fn run_as_server(path: &str) -> Result<(), Error> {
    IpcServer::serve(path, ServerRouter::route())?;

    Ok(())
}
