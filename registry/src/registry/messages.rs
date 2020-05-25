use actix::prelude::*;

use serde::{Deserialize, Serialize};

// Service Request
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "message", content = "data")]
pub enum RegistryRequest {
    List,
    Register(String),
}

impl actix::Message for RegistryRequest {
    type Result = RegistryResponse;
}

// Registry Response
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
pub enum RegistryResponse {
    Capabilities(Vec<String>),
    Registered,
    Error(String),
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Register {
    pub(super) capability: String,
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
