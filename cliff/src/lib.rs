pub extern crate actix;
pub extern crate async_trait;
pub extern crate failure;
pub extern crate futures;
pub extern crate serde;
pub extern crate tokio;
pub extern crate uuid;

pub mod client;
mod codec;
pub mod server;

pub use cliff_macros::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
