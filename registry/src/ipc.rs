use actix::prelude::*;

use failure::Error;

use super::registry::{ListCapabilities, Register, Registry, RegistryRequest, RegistryResponse};
use super::server::{IpcServer, Router};

pub struct ServerRouter {
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
    pub fn serve(path: &str) -> Result<(), Error> {
        IpcServer::serve(path, Self::route())?;

        Ok(())
    }

    fn route() -> Addr<Self> {
        let registry = Registry::start_default();

        Self { registry }.start()
    }
}

#[derive(Debug, Default)]
pub struct Printer;

impl Actor for Printer {
    type Context = Context<Self>;
}

impl Supervised for Printer {}

impl SystemService for Printer {}

impl Handler<RegistryResponse> for Printer {
    type Result = ();

    fn handle(&mut self, msg: RegistryResponse, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RegistryResponse::Registered => println!("Registered Service!"),
            RegistryResponse::Capabilities(capabilities) => {
                println!("--> Capabilities");
                for capability in capabilities {
                    println!("----> {}", capability);
                }
            }
            RegistryResponse::Error(e) => println!("Error: {:?}", e),
        }
    }
}
