use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::os::unix::net;

use actix::dev::ToEnvelope;
use actix::prelude::*;

use failure::{Error, ResultExt};

use serde::de::DeserializeOwned;
use serde::Serialize;

use tokio::io::WriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::stream::StreamExt;

use tokio_util::codec::FramedRead;

use tracing::{error, info, span, Level};

use super::codec::{Decoder, Encoder};

pub trait InboundMessage: Message + DeserializeOwned + Send + Unpin {}
pub trait OutboundMessage: fmt::Debug + Serialize + Send + Unpin {}

impl<M: Message + DeserializeOwned + Send + Unpin> InboundMessage for M {}
impl<M: fmt::Debug + Serialize + Send + Unpin> OutboundMessage for M {}

pub trait Router<In: InboundMessage>: Actor + Handler<In> {}

struct Session<In: InboundMessage, R: Router<In>>
where
    In::Result: OutboundMessage,
{
    router: Addr<R>,
    client: actix::io::FramedWrite<In::Result, WriteHalf<UnixStream>, Encoder<In::Result>>,
}

impl<In: InboundMessage + 'static, R: Router<In>> Actor for Session<In, R>
where
    In::Result: OutboundMessage,
{
    type Context = Context<Self>;
}

impl<In: InboundMessage + 'static, R: Router<In>> actix::io::WriteHandler<Error> for Session<In, R> where
    In::Result: OutboundMessage
{
}

impl<In: InboundMessage + 'static, R: Router<In>> StreamHandler<Result<In, Error>>
    for Session<In, R>
where
    In::Result: OutboundMessage,
    R::Context: ToEnvelope<R, In>,
{
    fn handle(&mut self, msg: Result<In, Error>, ctx: &mut Self::Context) {
        let span = span!(Level::TRACE, "Cliff Server StreamHandler");
        let _enter = span.enter();

        match msg {
            Ok(input) => self
                .router
                .send(input)
                .into_actor(self)
                .then(|res, act, _| {
                    match res {
                        Ok(res) => act.client.write(res),
                        Err(e) => error!("Error responding to request: {}", e.to_string()),
                    }
                    async {}.into_actor(act)
                })
                .wait(ctx),
            Err(e) => error!("Error handling msg: {}", e.to_string()),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct IpcConnect(pub UnixStream, pub net::SocketAddr);

pub struct IpcServer<In: InboundMessage, R: Router<In>> {
    inbound_message: PhantomData<In>,
    router: Addr<R>,
}

impl<In: InboundMessage + 'static, R: Router<In>> Actor for IpcServer<In, R> {
    type Context = Context<Self>;
}

impl<In: InboundMessage + 'static, R: Router<In>> Handler<IpcConnect> for IpcServer<In, R>
where
    In::Result: OutboundMessage,
    R::Context: ToEnvelope<R, In>,
{
    type Result = ();

    fn handle(&mut self, msg: IpcConnect, _ctx: &mut Self::Context) -> Self::Result {
        let router = self.router.clone();
        Session::create(move |ctx| {
            let (r, w) = tokio::io::split(msg.0);

            Session::add_stream(FramedRead::new(r, Decoder::<In>::new()), ctx);
            Session {
                router,
                client: actix::io::FramedWrite::new(w, Encoder::<In::Result>::new(), ctx),
            }
        });
    }
}

impl<In: InboundMessage + 'static, R: Router<In>> IpcServer<In, R>
where
    In::Result: OutboundMessage,
    R::Context: ToEnvelope<R, In>,
{
    fn new(router: Addr<R>) -> Self {
        Self {
            inbound_message: PhantomData,
            router,
        }
    }

    pub fn serve(path: &str, router: Addr<R>) -> Result<(), Error> {
        let span = span!(Level::TRACE, "Serving Router", path);
        let _enter = span.enter();

        let listener = Box::new(open_uds_listener(path).context("Couldn't open socket")?);

        IpcServer::create(move |ctx| {
            ctx.add_message_stream(Box::leak(listener).incoming().map(|stream| {
                let stream = stream.unwrap();
                let addr = stream.peer_addr().unwrap();
                IpcConnect(stream, addr)
            }));
            IpcServer::new(router)
        });

        Ok(())
    }
}

fn open_uds_listener(path: &str) -> Result<UnixListener, Error> {
    match UnixListener::bind(&path) {
        Ok(l) => Ok(l),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            // 1. Handle cases where file exists
            // TODO: Handle it more gracefully (Ask user whether to force or abort)
            info!("A connection file already exists. Removing it.");
            fs::remove_file(&path)?;

            UnixListener::bind(&path).map_err(Error::from)
        }
        Err(e) => Err(Error::from(e)),
    }
}
