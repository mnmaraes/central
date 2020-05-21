use std::os::unix::net;

use actix::prelude::*;

use tokio::io::WriteHalf;
use tokio::net::UnixStream;

use tokio_util::codec::FramedRead;

use failure::Error;

use super::codec::Codec;
// TODO: Abstract Registry Away To create a generic IPC Server actor
use super::registry::{ListCapabilities, Register, Registry, RegistryRequest, RegistryResponse};

type ServiceCodec = Codec<RegistryRequest, RegistryResponse>;
type RegistryCodec = Codec<RegistryResponse, RegistryRequest>;

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
                self.registry
                    .send(Register::new(capability))
                    .into_actor(self)
                    .then(|res, act, _| {
                        match res {
                            Ok(_) => act.service.write(RegistryResponse::Registered),
                            _ => println!("Error listing capabilities"),
                        }
                        async {}.into_actor(act)
                    })
                    .wait(ctx);
            }
            Err(e) => println!("Error handling msg: {}", e.to_string()),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct IpcConnect(pub UnixStream, pub net::SocketAddr);

pub struct IpcServer {
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

            RegistrationSession::add_stream(FramedRead::new(r, ServiceCodec::new()), ctx);
            RegistrationSession {
                registry,
                service: actix::io::FramedWrite::new(w, ServiceCodec::new(), ctx),
            }
        });
    }
}

impl IpcServer {
    pub fn new(registry: Addr<Registry>) -> Self {
        Self { registry }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum ClientRequest {
    Register(String),
    List,
}

pub struct IpcClient {
    framed: actix::io::FramedWrite<RegistryRequest, WriteHalf<UnixStream>, RegistryCodec>,
}

impl actix::io::WriteHandler<Error> for IpcClient {}

impl Actor for IpcClient {
    type Context = Context<Self>;
}

impl Handler<ClientRequest> for IpcClient {
    type Result = ();

    fn handle(&mut self, msg: ClientRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ClientRequest::Register(capability) => {
                self.framed.write(RegistryRequest::Register(capability))
            }
            ClientRequest::List => self.framed.write(RegistryRequest::List),
        }
    }
}

impl IpcClient {
    pub fn new(stream: UnixStream, ctx: &mut Context<Self>) -> Self {
        let (r, w) = tokio::io::split(stream);
        ctx.add_stream(FramedRead::new(r, RegistryCodec::new()));
        Self {
            framed: actix::io::FramedWrite::new(w, RegistryCodec::new(), ctx),
        }
    }
}

impl StreamHandler<Result<RegistryResponse, Error>> for IpcClient {
    fn handle(&mut self, item: Result<RegistryResponse, Error>, _ctx: &mut Self::Context) {
        match item {
            Ok(RegistryResponse::Capabilities(capabilities)) => {
                println!("\nRegistered Capabilities: ");
                for capability in capabilities {
                    println!("{}", capability);
                }
                println!();
            }
            Ok(RegistryResponse::Registered) => {
                println!("Capability Registered");
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}
