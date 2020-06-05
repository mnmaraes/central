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
