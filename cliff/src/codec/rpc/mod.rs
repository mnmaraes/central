pub mod msgpack;

pub enum RpcMessageType {
    Request,
    Notification,
    Response,
    Error,
}

pub trait RpcMessage {
    fn rpc_message_type(&self) -> RpcMessageType;
}
