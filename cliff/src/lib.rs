pub extern crate codec;
pub extern crate actix;
pub extern crate async_trait;
pub extern crate failure;
pub extern crate futures;
pub extern crate rand;
pub extern crate serde;
pub extern crate tokio;

pub mod client;
pub mod server;

pub mod rpc {
    pub use super::codec::{RpcMessage, RpcMessageType};
}

pub use macros::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
