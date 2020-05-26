use actix::dev::{MessageResponse, ResponseChannel};
use actix::prelude::*;

use serde::{Deserialize, Serialize};

// Service Request
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "message", content = "data")]
pub enum RegistryRequest {
    Require {
        id: String,
        capability: String,
    },
    Register {
        id: String,
        capability: String,
        address: String,
    },
}

impl actix::Message for RegistryRequest {
    type Result = RegistryResponse;
}

// Registry Response
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
pub enum RegistryResponse {
    Capability { id: String, address: String },
    Registered { id: String },
    Error(String),
}

impl<A, M> MessageResponse<A, M> for RegistryResponse
where
    A: Actor,
    M: Message<Result = RegistryResponse>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}
