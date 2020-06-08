use actix::prelude::*;

use failure::{Error, ResultExt};

use im::HashMap;

use cliff::client::IpcClient;
use cliff::server::IpcServer;
use cliff::{client, router};

pub use registry_macros::provide;

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
        Require { capability: String } -> {
            let addr = self.providers.get(&capability);
        } => [
            let Some(addr) = addr => Capability [String] { address: addr.clone() },
            => Error [String] { description: "Capability Not Found".into() }
        ],
        Register { capability: String, address: String } -> {
            self.providers.insert(capability, address);
        } => Registered
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
            Register { capability: String, address: String } wait
        ],
        response_mapping => [
            Registered => [ () ]
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
            Require { capability: String } wait String
        ],
        response_mapping => [
            Capability { address } => [
                String: address
            ]
        ]
    }
}

impl InterfaceClient {
    #[allow(dead_code)]
    pub async fn connect_default() -> Result<Addr<Self>, Error> {
        InterfaceClient::connect("/tmp/central.registry").await
    }
}
