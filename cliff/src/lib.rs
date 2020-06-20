pub extern crate actix;
pub extern crate async_trait;
pub extern crate failure;
pub extern crate futures;
pub extern crate rand;
pub extern crate serde;
pub extern crate tokio;

pub mod client;
mod codec;
pub mod server;

pub use macros::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
