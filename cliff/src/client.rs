use actix::prelude::*;

use failure::Error;

use serde::de::DeserializeOwned;
use serde::Serialize;

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::UnixStream;

use tokio_util::codec::FramedRead;

use super::codec::{Decoder, Encoder, RpcMessage};

#[async_trait::async_trait]
pub trait IpcClient: Actor {
    async fn connect(path: &str) -> Result<Addr<Self>, Error>;
}

pub trait InterfaceMessage: Message + RpcMessage + Serialize + Unpin {}

impl<M: Message + RpcMessage + Serialize + Unpin> InterfaceMessage for M {}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InterfaceRequest<I>(pub I);

pub struct WriteInterface<I: InterfaceMessage> {
    framed: actix::io::FramedWrite<I, WriteHalf<UnixStream>, Encoder<I>>,
}

impl<I: InterfaceMessage + 'static> Actor for WriteInterface<I> {
    type Context = Context<Self>;
}

impl<I: InterfaceMessage + 'static> Handler<InterfaceRequest<I>> for WriteInterface<I> {
    type Result = ();

    fn handle(&mut self, msg: InterfaceRequest<I>, _ctx: &mut Self::Context) -> Self::Result {
        self.framed.write(msg.0);
    }
}

impl<I: InterfaceMessage + 'static> actix::io::WriteHandler<Error> for WriteInterface<I> {}

impl<I: InterfaceMessage + 'static> WriteInterface<I> {
    pub async fn attach(w: WriteHalf<UnixStream>) -> Result<Addr<WriteInterface<I>>, Error> {
        let addr = Self::create(|ctx| Self {
            framed: actix::io::FramedWrite::new(w, Encoder::<I>::default(), ctx),
        });

        Ok(addr)
    }
}

pub trait InterfaceResponse: Message + DeserializeOwned + Unpin + Send {}

impl<M: Message + DeserializeOwned + Unpin + Send> InterfaceResponse for M {}

pub trait Delegate<I: InterfaceResponse>: Actor {
    fn listen(r: ReadHalf<UnixStream>, ctx: &mut Self::Context);
}

impl<I: InterfaceResponse + 'static, D: Actor + StreamHandler<Result<I, Error>>> Delegate<I> for D
where
    D: Actor<Context = Context<D>>,
{
    fn listen(r: ReadHalf<UnixStream>, ctx: &mut Self::Context) {
        ctx.add_stream(FramedRead::new(r, Decoder::<I>::default()));
    }
}
