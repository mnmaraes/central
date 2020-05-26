use actix::prelude::*;

use im::HashMap;

use super::messages::{RegistryRequest, RegistryResponse};

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

impl Handler<RegistryRequest> for Registry {
    type Result = RegistryResponse;

    fn handle(&mut self, msg: RegistryRequest, _ctx: &mut Self::Context) -> Self::Result {
        use RegistryRequest::*;

        let res = match msg {
            Require { id, capability } => match self.providers.get(&capability) {
                Some(addr) => RegistryResponse::Capability {
                    id,
                    address: addr.clone(),
                },
                None => RegistryResponse::Error("Capability Not Found".to_string()),
            },
            Register {
                id,
                capability,
                address,
            } => {
                self.providers.insert(capability.clone(), address.clone());
                RegistryResponse::Registered { id }
            }
        };

        res
    }
}
