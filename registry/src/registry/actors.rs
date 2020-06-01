use actix::prelude::*;

use im::HashMap;

use cliff::router;

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
        Require { id: String, capability: String } -> {
            let addr = self.providers.get(&capability);
        } => [
            let Some(addr) = addr => Capability [String, String] { id, address: addr.clone() },
            => Error [String] { description: "Capability Not Found".into() }
        ],
        Register { id: String, capability: String, address: String } -> {
            self.providers.insert(capability, address);
        } => Registered [String] { id }
    ]
}

