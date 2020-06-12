pub extern crate actix;
pub extern crate actix_rt;
pub extern crate cliff;
pub extern crate failure;
pub extern crate tokio;
pub extern crate uuid;

use actix::prelude::*;

use failure::{format_err, Error, ResultExt};

use tracing::info;

use im::HashMap;

use cliff::client::IpcClient;
use cliff::server::IpcServer;
use cliff::{client, router};

pub use registry_macros::*;

pub struct Registry {
    providers: HashMap<String, String>,
}

impl Actor for Registry {
    type Context = Context<Self>;
}

impl Default for Registry {
    fn default() -> Self {
        Registry {
            providers: HashMap::new(),
        }
    }
}

router! {
    Registry [
        // Interface
        Require { capability: String } -> {
            info!("Client Requiring Capability: {}", capability);
            let addr = self.providers.get(&capability);
        } => [
            let Some(addr) = addr => Capability [String] { address: addr.clone() },
            => Error [String] { description: "Capability Not Found".into() }
        ],
        // Provider
        Register { capability: String, address: String } -> {
            info!("Client Registering Capability: {}", capability);
            self.providers.insert(capability, address);
        } => Success,
        Deregister { capability: String } -> {
            info!("Client Deregistering Capability: {}", capability);
            self.providers.remove(&capability);
        } => Success,
        // Status
        Check => Alive
    ]
}

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

client! {
    Registry named Provider {
        actions => [
            Register { capability: String, address: String } wait,
            Deregister { capability: String } wait
        ],
        response_mapping => [
            Success => [ () ]
        ]
    }
}

impl ProviderClient {
    #[allow(dead_code)]
    pub async fn register_default(capability: &str, address: &str) -> Result<Addr<Self>, Error> {
        let path = "/tmp/central.registry";

        let addr = ProviderClient::connect(path).await?;

        addr.send(Register {
            capability: capability.to_string(),
            address: address.to_string(),
        })
        .await?;

        Ok(addr)
    }

    #[allow(dead_code)]
    pub async fn connect_default() -> Result<Addr<Self>, Error> {
        let path = "/tmp/central.registry";

        let addr = ProviderClient::connect(path).await?;

        Ok(addr)
    }
}

client! {
    Registry named Interface {
        actions => [
            Require { capability: String } wait Result<String, Error>
        ],
        response_mapping => [
            Capability { address } => [
                Result<String, Error>: Ok(address)
            ],
            Error { description } => [
                Result<String, Error>: Err(format_err!("Error: {}", description))
            ]
        ]
    }
}

client! {
    Registry named Status {
        actions => [
            Check wait,
        ],
        response_mapping => [
            Alive => [ () ]
        ]
    }
}

impl InterfaceClient {
    #[allow(dead_code)]
    pub async fn connect_default() -> Result<Addr<Self>, Error> {
        InterfaceClient::connect("/tmp/central.registry").await
    }
}

impl StatusClient {
    #[allow(dead_code)]
    pub async fn check_default() -> Result<(), Error> {
        let path = "/tmp/central.registry";

        let addr = StatusClient::connect(path).await?;

        addr.send(Check).await?;

        Ok(())
    }
}
