use actix::prelude::*;

use im::HashMap;

use super::messages::{RegistryRequest, RegistryResponse};

pub struct Registry {
    providers: HashMap<String, Addr<Provider>>,
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
            List => RegistryResponse::Capabilities(
                self.providers.keys().map(|s| s.to_string()).collect(),
            ),
            Register(capability) => {
                match self.providers.insert(capability.clone(), Provider.start()) {
                    Some(_) => RegistryResponse::Registered,
                    None => RegistryResponse::Error(format!(
                        "Error registering capability: {}",
                        capability
                    )),
                }
            }
        };

        res
    }
}

struct Provider;

impl Actor for Provider {
    type Context = Context<Self>;
}
