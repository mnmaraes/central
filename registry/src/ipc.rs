use std::collections::HashMap;

use actix::prelude::*;

use failure::{Error, ResultExt};

use futures::FutureExt;

use cliff::client::{Delegate, InterfaceRequest, WriteInterface};
use cliff::server::{IpcServer, Router};

use tokio::net::UnixStream;
use tokio::sync::oneshot;

use uuid::Uuid;

use super::registry::{Registry, RegistryRequest, RegistryResponse};

// Registry extension
impl Router<RegistryRequest> for Registry {}

impl Registry {
    #[allow(dead_code)]
    pub fn serve(path: &str) -> Result<(), Error> {
        IpcServer::serve(path, Self::start_default())?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn serve_default() -> Result<(), Error> {
        let path = "/tmp/central.registry";

        IpcServer::serve(path, Self::start_default())
            .context(format!("Error serving on ipc path: {}", path))?;

        Ok(())
    }
}

// Provider Client
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct Register {
    pub capability: String,
    pub address: String,
}

pub struct ProviderClient {
    writer: Addr<WriteInterface<RegistryRequest>>,
    futures: HashMap<String, oneshot::Sender<()>>,
}

impl Actor for ProviderClient {
    type Context = Context<Self>;
}

impl Handler<Register> for ProviderClient {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: Register, _ctx: &mut Self::Context) -> Self::Result {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        println!("Sent registration");
        self.futures.insert(id.clone(), tx);

        Box::pin(
            self.writer
                .send(InterfaceRequest(RegistryRequest::Register {
                    id,
                    capability: msg.capability,
                    address: msg.address,
                }))
                .then(|_res| async move {
                    println!("Awaiting registration return");
                    rx.await.unwrap();
                    println!("Done awaiting");
                }),
        )
    }
}

impl StreamHandler<Result<RegistryResponse, Error>> for ProviderClient {
    fn handle(&mut self, item: Result<RegistryResponse, Error>, _ctx: &mut Self::Context) {
        match item {
            Ok(RegistryResponse::Registered { id }) => {
                println!("Registered id: {}", id);
                if let Some(tx) = self.futures.remove(&id) {
                    println!("Sending registration confirmation");
                    tx.send(()).unwrap();
                }
            }
            Ok(RegistryResponse::Error(e)) => {
                println!("Error: {}", e);
            }
            Ok(c) => {
                println!("Unhandled Case: {:?}", c);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

impl ProviderClient {
    #[allow(dead_code)]
    pub async fn connect(path: &str) -> Result<Addr<Self>, Error> {
        let stream = UnixStream::connect(path).await?;
        let (r, w) = tokio::io::split(stream);

        let writer = WriteInterface::attach(w).await?;

        let addr = ProviderClient::create(|ctx| {
            ProviderClient::listen(r, ctx);

            ProviderClient {
                writer,
                futures: HashMap::new(),
            }
        });

        Ok(addr)
    }

    #[allow(dead_code)]
    pub async fn register_default(capability: &str, address: &str) -> Result<Addr<Self>, Error> {
        let path = "/tmp/central.registry";

        let stream = UnixStream::connect(path).await?;
        let (r, w) = tokio::io::split(stream);

        let writer = WriteInterface::attach(w).await?;

        let addr = ProviderClient::create(|ctx| {
            ProviderClient::listen(r, ctx);

            ProviderClient {
                writer,
                futures: HashMap::new(),
            }
        });

        addr.send(Register {
            capability: capability.to_string(),
            address: address.to_string(),
        })
        .await?;

        Ok(addr)
    }
}

// Interface Client
#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct Require(pub String);

pub struct InterfaceClient {
    writer: Addr<WriteInterface<RegistryRequest>>,
    futures: HashMap<String, oneshot::Sender<String>>,
}

impl Actor for InterfaceClient {
    type Context = Context<Self>;
}

impl Handler<Require> for InterfaceClient {
    type Result = ResponseFuture<String>;

    fn handle(&mut self, msg: Require, _ctx: &mut Self::Context) -> Self::Result {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        self.futures.insert(id.clone(), tx);

        Box::pin(
            self.writer
                .send(InterfaceRequest(RegistryRequest::Require {
                    id,
                    capability: msg.0,
                }))
                .then(|_res| async move { rx.await.unwrap() }),
        )
    }
}

impl StreamHandler<Result<RegistryResponse, Error>> for InterfaceClient {
    fn handle(&mut self, item: Result<RegistryResponse, Error>, _ctx: &mut Self::Context) {
        match item {
            Ok(RegistryResponse::Capability { id, address }) => {
                println!("Capability Address: {}", address);
                if let Some(tx) = self.futures.remove(&id) {
                    println!("Sending registration confirmation");
                    tx.send(address).unwrap();
                }
            }
            Ok(RegistryResponse::Error(e)) => {
                println!("Error: {}", e);
            }
            Ok(c) => {
                println!("Unhandled Case: {:?}", c);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

impl InterfaceClient {
    #[allow(dead_code)]
    pub async fn connect(path: &str) -> Result<Addr<Self>, Error> {
        let stream = UnixStream::connect(path).await?;
        let (r, w) = tokio::io::split(stream);

        let writer = WriteInterface::attach(w).await?;

        let addr = InterfaceClient::create(|ctx| {
            InterfaceClient::listen(r, ctx);

            InterfaceClient {
                writer,
                futures: HashMap::new(),
            }
        });

        Ok(addr)
    }

    #[allow(dead_code)]
    pub async fn connect_default() -> Result<Addr<Self>, Error> {
        InterfaceClient::connect("/tmp/central.registry").await
    }
}
