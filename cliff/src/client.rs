use std::fmt;
use std::marker::PhantomData;

use actix::prelude::*;

use failure::Error;

use serde::de::DeserializeOwned;
use serde::Serialize;

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::UnixStream;

use tokio_util::codec::FramedRead;

use super::codec::{Decoder, Encoder};

pub trait InterfaceMessage: Message + fmt::Debug + Serialize + Unpin {}

impl<M: Message + fmt::Debug + Serialize + Unpin> InterfaceMessage for M {}

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
            framed: actix::io::FramedWrite::new(w, Encoder::<I>::new(), ctx),
        });

        Ok(addr)
    }
}

pub trait InterfaceResponse: Message + DeserializeOwned + Unpin + Send {}

impl<M: Message + DeserializeOwned + Unpin + Send> InterfaceResponse for M {}

// Basic Routing
pub trait Delegate<I: InterfaceResponse>: Actor {
    fn listen(r: ReadHalf<UnixStream>, ctx: &mut Self::Context);
}

impl<I: InterfaceResponse + 'static, D: Actor + StreamHandler<Result<I, Error>>> Delegate<I> for D
where
    D: Actor<Context = Context<D>>,
{
    fn listen(r: ReadHalf<UnixStream>, ctx: &mut Self::Context) {
        ctx.add_stream(FramedRead::new(r, Decoder::<I>::new()));
    }
}

// System Responder
pub trait SystemResponder<I: InterfaceResponse>:
    Actor + actix::Supervised + SystemService + Handler<I>
{
    fn subscribe(r: ReadHalf<UnixStream>) -> Result<(), Error>;
}

impl<I: InterfaceResponse + 'static, R: Actor + actix::Supervised + SystemService + Handler<I>>
    SystemResponder<I> for R
where
    I::Result: Send,
{
    fn subscribe(r: ReadHalf<UnixStream>) -> Result<(), Error> {
        Arbiter::spawn(async move {
            SystemForwardClient::<I, R>::create(|ctx| {
                ctx.add_stream(FramedRead::new(r, Decoder::<I>::new()));
                SystemForwardClient::new()
            });
        });

        Ok(())
    }
}

// Forwarding Client
struct SystemForwardClient<I: InterfaceResponse, R: SystemResponder<I>> {
    i: PhantomData<I>,
    r: PhantomData<R>,
}

impl<I: InterfaceResponse + 'static, R: SystemResponder<I>> Actor for SystemForwardClient<I, R> {
    type Context = Context<Self>;
}

impl<I: InterfaceResponse + 'static, R: SystemResponder<I>> StreamHandler<Result<I, Error>>
    for SystemForwardClient<I, R>
where
    I::Result: Send,
{
    fn handle(&mut self, item: Result<I, Error>, _ctx: &mut Self::Context) {
        match item {
            Ok(msg) => {
                let act = R::from_registry();
                act.do_send(msg);
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}

impl<I: InterfaceResponse, R: SystemResponder<I>> SystemForwardClient<I, R> {
    pub fn new() -> Self {
        Self {
            i: PhantomData,
            r: PhantomData,
        }
    }
}
