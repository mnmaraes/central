mod parsing;

use std::marker::PhantomData;

use bytes::{buf::*, BytesMut};

use failure::Error;

use serde::de::DeserializeOwned;
use serde::Serialize;

use tracing::info;

use serde_json as json;

use crate::{RpcMessage, RpcMessageType};

use parsing::*;

pub struct Encoder<Out> {
    outbound_message: PhantomData<Out>,
}

impl<Out> Default for Encoder<Out> {
    fn default() -> Self {
        Self {
            outbound_message: PhantomData,
        }
    }
}

impl<Out: Serialize + RpcMessage> tokio_util::codec::Encoder<Out> for Encoder<Out> {
    type Error = Error;

    fn encode(&mut self, msg: Out, dst: &mut BytesMut) -> Result<(), Error> {
        let val = json::to_value(&msg)?;
        let msg = match msg.rpc_message_type() {
            RpcMessageType::Request => parsed_encoded_request(&val)?,
            RpcMessageType::Notification => parsed_encoded_notification(&val)?,
            RpcMessageType::Response => parsed_encoded_response(&val)?,
            RpcMessageType::Error => parsed_encoded_error(&val)?,
        };
        info!("Encoding: {:?}", msg);

        let msg: &[u8] = &json::to_vec(&msg)?;
        dst.put(msg);

        Ok(())
    }
}

pub struct Decoder<In> {
    inbound_message: PhantomData<In>,
}

impl<In> Default for Decoder<In> {
    fn default() -> Self {
        Self {
            inbound_message: PhantomData,
        }
    }
}

impl<In: DeserializeOwned> tokio_util::codec::Decoder for Decoder<In> {
    type Item = In;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Error> {
        if src.is_empty() {
            return Ok(None);
        }
        let mut de: json::StreamDeserializer<_, json::Value> =
            json::Deserializer::from_reader(src.reader()).into_iter();
        match de.next() {
            Some(Ok(v)) => {
                info!("Decoding: {:?}", v);
                Ok(Some(parse_decoded(&v)?))
            }
            Some(Err(e)) if e.is_eof() => Ok(None),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}
