use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, BytesMut};

use tokio_util::codec::{Decoder, Encoder};

use failure::{Error, ResultExt};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json as json;

use super::registry::{RegistryRequest, RegistryResponse};

fn json_string_encode(msg: String, dst: &mut BytesMut) -> Result<(), Error> {
    let msg_ref: &[u8] = msg.as_ref();

    dst.reserve(msg_ref.len() + 2);
    dst.put_u16(msg_ref.len() as u16);
    dst.put(msg_ref);

    Ok(())
}

fn json_decode<T: DeserializeOwned>(src: &mut BytesMut) -> Result<Option<T>, Error> {
    let size = {
        if src.len() < 2 {
            return Ok(None);
        }
        BigEndian::read_u16(src.as_ref()) as usize
    };

    if src.len() >= size + 2 {
        src.advance(2);
        let buf = src.split_to(size);
        Ok(Some(json::from_slice::<T>(&buf)?))
    } else {
        Ok(None)
    }
}

// Service => Registry
pub struct ServiceCodec;

impl Decoder for ServiceCodec {
    type Item = RegistryRequest;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        json_decode(src)
    }
}

impl Encoder<RegistryResponse> for ServiceCodec {
    type Error = Error;

    fn encode(&mut self, msg: RegistryResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).context(format!("Couldn't Encode Message: {:?}", msg))?;
        json_string_encode(msg, dst)
    }
}

// Registry => Service
pub struct RegistryCodec;

impl Decoder for RegistryCodec {
    type Item = RegistryResponse;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        json_decode(src)
    }
}

impl Encoder<RegistryRequest> for RegistryCodec {
    type Error = Error;

    fn encode(&mut self, msg: RegistryRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).context(format!("Couldn't Encode Message: {:?}", msg))?;
        json_string_encode(msg, dst)
    }
}
