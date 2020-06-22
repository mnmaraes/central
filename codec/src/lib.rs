mod rpc;

pub use rpc::{Encoder, Decoder};

pub enum RpcMessageType {
    Request,
    Notification,
    Response,
    Error,
}

pub trait RpcMessage {
    fn rpc_message_type(&self) -> RpcMessageType;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
