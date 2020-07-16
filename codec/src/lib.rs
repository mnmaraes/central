mod rpc;

pub use rpc::{Decoder, Encoder};

pub enum RpcMessageType {
    Request,
    Notification,
    Response,
    Error,
}

pub trait RpcMessage {
    fn rpc_message_type(&self) -> RpcMessageType;
}

impl<O, E> RpcMessage for Result<O, E> {
    fn rpc_message_type(&self) -> RpcMessageType {
        match self {
            Ok(_) => RpcMessageType::Response,
            Err(_) => RpcMessageType::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
