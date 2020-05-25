use actix::prelude::*;

use failure::Error;

use super::registry::{Registry, RegistryRequest, RegistryResponse};
use cliff::server::{IpcServer, Router};

impl Router<RegistryRequest> for Registry {}

impl Registry {
    pub fn serve(path: &str) -> Result<(), Error> {
        IpcServer::serve(path, Self::start_default())?;

        Ok(())
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
