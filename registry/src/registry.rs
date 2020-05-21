use actix::prelude::*;

use im::HashMap;

use serde::{Deserialize, Serialize};

// Service Request
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
pub enum RegistryRequest {
    List,
    Register(String),
}

// Registry Response
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
pub enum RegistryResponse {
    Capabilities(Vec<String>),
    Registered,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Register {
    capability: String,
}

impl Register {
    pub fn new(capability: String) -> Self {
        Self { capability }
    }
}

pub struct ListCapabilities;

impl actix::Message for ListCapabilities {
    type Result = Vec<String>;
}

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

impl Handler<Register> for Registry {
    type Result = ();

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        self.providers.insert(msg.capability, Provider.start());
    }
}

impl Handler<ListCapabilities> for Registry {
    type Result = MessageResult<ListCapabilities>;

    fn handle(&mut self, _: ListCapabilities, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.providers.keys().map(|s| s.to_string()).collect())
    }
}

struct Provider;

impl Actor for Provider {
    type Context = Context<Self>;
}
