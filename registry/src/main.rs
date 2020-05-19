use std::os::unix::net;
use std::{fs, io::ErrorKind};

use im::HashMap;

use actix::prelude::*;

use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, BytesMut};

use tokio::io::WriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::stream::StreamExt;

use tokio_util::codec::{Decoder, Encoder, FramedRead};

use failure::{Error, ResultExt};

use serde::{Deserialize, Serialize};
use serde_json as json;

#[derive(Serialize, Deserialize, Debug)]
struct Note {
    body: String,
}

struct Provider;

impl Actor for Provider {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct Register {
    capability: String,
}

struct ListCapabilities;

impl actix::Message for ListCapabilities {
    type Result = Vec<String>;
}

struct Registry {
    providers: HashMap<String, Addr<Provider>>,
}

impl Actor for Registry {
    type Context = Context<Self>;
}

impl Default for Registry {
    fn default() -> Self {
        Registry {
            providers: HashMap::new(),
        }
    }
}

impl Handler<Register> for Registry {
    type Result = ();

    fn handle(&mut self, msg: Register, _: &mut Self::Context) -> Self::Result {
        self.providers.insert(msg.capability, Provider.start());
    }
}

impl Handler<ListCapabilities> for Registry {
    type Result = MessageResult<ListCapabilities>;

    fn handle(&mut self, _: ListCapabilities, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.providers.keys().map(|s| s.to_string()).collect())
    }
}

// Service Request
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
enum RegistryRequest {
    List,
    Register(String),
}

// Registry Response
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
enum RegistryResponse {
    Capabilities(Vec<String>),
    Registered,
}

// Service => Registry
struct ServiceCodec;

impl Decoder for ServiceCodec {
    type Item = RegistryRequest;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.advance(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<RegistryRequest>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<RegistryResponse> for ServiceCodec {
    type Error = Error;

    fn encode(&mut self, msg: RegistryResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).context(format!("Couldn't Encode Message: {:?}", msg))?;
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

// Registry => Service
struct RegistryCodec;

impl Decoder for RegistryCodec {
    type Item = RegistryResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.advance(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<RegistryResponse>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<RegistryRequest> for RegistryCodec {
    type Error = Error;

    fn encode(&mut self, msg: RegistryRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).context(format!("Couldn't Encode Message: {:?}", msg))?;
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

struct RegistrationSession {
    registry: Addr<Registry>,
    service: actix::io::FramedWrite<RegistryResponse, WriteHalf<UnixStream>, ServiceCodec>,
}

impl Actor for RegistrationSession {
    type Context = Context<Self>;
}

impl actix::io::WriteHandler<Error> for RegistrationSession {}

impl StreamHandler<Result<RegistryRequest, Error>> for RegistrationSession {
    fn handle(&mut self, msg: Result<RegistryRequest, Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(RegistryRequest::List) => {
                self.registry
                    .send(ListCapabilities)
                    .into_actor(self)
                    .then(|res, act, _| {
                        match res {
                            Ok(capabilities) => act
                                .service
                                .write(RegistryResponse::Capabilities(capabilities)),
                            _ => println!("Error listing capabilities"),
                        }
                        async {}.into_actor(act)
                    })
                    .wait(ctx);
            }
            Ok(RegistryRequest::Register(capability)) => {
                self.registry.do_send(Register { capability });
            }
            Err(e) => println!("Error handling msg: {}", e.to_string()),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct IpcConnect(pub UnixStream, pub net::SocketAddr);

struct IpcServer {
    registry: Addr<Registry>,
}

impl Actor for IpcServer {
    type Context = Context<Self>;
}

impl Handler<IpcConnect> for IpcServer {
    type Result = ();

    fn handle(&mut self, msg: IpcConnect, _ctx: &mut Self::Context) -> Self::Result {
        let registry = self.registry.clone();
        RegistrationSession::create(move |ctx| {
            let (r, w) = tokio::io::split(msg.0);

            RegistrationSession::add_stream(FramedRead::new(r, ServiceCodec), ctx);
            RegistrationSession {
                registry,
                service: actix::io::FramedWrite::new(w, ServiceCodec, ctx),
            }
        });
    }
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let registry = Registry::start_default();

    let listener = Box::new(open_uds_listener().context("Couldn't open socket")?);

    IpcServer::create(move |ctx| {
        ctx.add_message_stream(Box::leak(listener).incoming().map(|stream| {
            let stream = stream.unwrap();
            let addr = stream.peer_addr().unwrap();
            IpcConnect(stream, addr)
        }));
        IpcServer { registry }
    });

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}

fn open_uds_listener() -> Result<UnixListener, Error> {
    let path = "/tmp/central.registry";
    match UnixListener::bind(&path) {
        Ok(l) => Ok(l),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            // 1. Handle cases where file exists
            // TODO: Handle it more gracefully (Ask user whether to force or abort)
            println!("A connection file already exists. Removing it.");
            fs::remove_file(&path)?;

            UnixListener::bind(&path).map_err(Error::from)
        }
        Err(e) => Err(Error::from(e)),
    }
}
