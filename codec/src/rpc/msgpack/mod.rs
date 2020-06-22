mod parsing;

use std::marker::PhantomData;

use bytes::{buf::*, BytesMut};

use failure::{format_err, Error};

use serde::de::DeserializeOwned;
use serde::Serialize;

use rmp_serde as rmps;

use tracing::info;

use crate::{RpcMessage, RpcMessageType};

use parsing::*;

#[derive(Default)]
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

fn get_rmp_value<Msg: Serialize>(msg: &Msg) -> Result<rmpv::Value, Error> {
    let mut vec = Vec::new();
    let mut serializer = rmps::encode::Serializer::new(&mut vec)
        .with_string_variants()
        .with_struct_map();
    msg.serialize(&mut serializer)?;

    Ok(rmpv::decode::read_value(&mut vec.reader())?)
}

impl<Out: Serialize + RpcMessage> tokio_util::codec::Encoder<Out> for Encoder<Out> {
    type Error = Error;

    fn encode(&mut self, msg: Out, dst: &mut BytesMut) -> Result<(), Error> {
        let value = get_rmp_value(&msg)?;
        let to_encode = match msg.rpc_message_type() {
            RpcMessageType::Request => parse_encoded_request_value(&value)?,
            RpcMessageType::Notification => parse_encoded_notification_value(&value)?,
            RpcMessageType::Response => parse_encoded_response_value(&value)?,
            RpcMessageType::Error => parse_encoded_error_value(&value)?,
        };

        info!("Encoding: {:?}", to_encode);

        rmpv::encode::write_value(&mut dst.writer(), &to_encode)?;

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

        let size = match get_value_size(src) {
            Some(size) if src.len() >= size => size,
            Some(size) => {
                let needed = size - src.len() + 16;
                src.reserve(needed);

                return Ok(None);
            }
            None => return Ok(None),
        };
        let to_parse = src.split_to(size);
        info!("Decoding: {:?}", to_parse.to_vec());
        // TODO: Decode the rpc format passed back
        let value = parse_array(&rmpv::decode::read_value(&mut to_parse.reader())?)?;

        let parsed = match parse_int(&value[0])? {
            0 => parse_decoded_request_value(&value)?,
            1 => parse_decoded_result_value(&value)?,
            2 => parse_decoded_notification_value(&value)?,
            _ => return Err(format_err!("Unexpected Message type")),
        };

        Ok(parsed)
    }
}

// Decoding
enum DataSize {
    S8,
    S16,
    S32,
}

fn get_be_size(buffer: &[u8], size_type: DataSize) -> usize {
    use DataSize::*;
    match size_type {
        S8 => buffer[1] as usize,
        S16 => u16::from_be_bytes([buffer[1], buffer[2]]) as usize,
        S32 => u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]) as usize,
    }
}

fn size_compound(buffer: &[u8], count: usize, skip: usize) -> Option<usize> {
    let mut sized_elements = 0;
    let mut size = skip;

    while sized_elements < count {
        if buffer.len() <= size {
            return None;
        }

        match get_value_size(&buffer[size..]) {
            Some(s) => size += s,
            None => return None,
        }

        sized_elements += 1;
    }

    Some(size)
}

fn get_value_size(buffer: &[u8]) -> Option<usize> {
    use DataSize::*;
    assert_ne!(
        buffer.len(),
        0,
        "Invariant Violation: Can't size message without bytes"
    );

    // Match simple sized values
    match buffer[0] {
        // *Number*
        // fixint | (nil or boolean) | negative fixint
        0..=0x7f | 0xc0..=0xc3 | 0xe0..=0xff => Some(1),
        // 8-bit: uint | int
        0xcc | 0xd0 => Some(2),
        // 16-bit: uint | int
        0xcd | 0xd1 => Some(3),
        // 32-bit: float | uint | int
        0xca | 0xce | 0xd2 => Some(5),
        // 64-bit: float | uint | int
        0xcb | 0xcf | 0xd3 => Some(9),
        //
        // *String or Binary*
        // fixstring
        0xa0..=0xbf => Some(1 + (buffer[0] & 0x1f) as usize), // 101XXXXX & 00011111
        // 8-bit size: str | bin
        0xd9 | 0xc4 if buffer.len() < 2 => None,
        0xd9 | 0xc4 => Some(2 + get_be_size(&buffer, S8)),
        // 16-bit size: str | bin
        0xda | 0xc5 if buffer.len() < 3 => None,
        0xda | 0xc5 => Some(3 + get_be_size(&buffer, S16)),
        // 32-but size: str | bin
        0xdb | 0xc6 if buffer.len() < 5 => None,
        0xdb | 0xc6 => Some(5 + get_be_size(&buffer, S32)),
        //
        // *Ext*
        // fixext(1)
        0xd4 => Some(3),
        // fixext(2)
        0xd5 => Some(4),
        // fixext(4)
        0xd6 => Some(6),
        // fixext(8)
        0xd7 => Some(10),
        // fixext(16)
        0xd8 => Some(18),
        // ext(8)
        0xc7 if buffer.len() < 2 => None,
        0xc7 => Some(3 + get_be_size(&buffer, S8)),
        // ext(16)
        0xc8 if buffer.len() < 3 => None,
        0xc8 => Some(4 + get_be_size(&buffer, S16)),
        // ext(32)
        0xc9 if buffer.len() < 5 => None,
        0xc9 => Some(6 + get_be_size(&buffer, S32)),
        //
        // *Array*
        // fixarray
        0x90..=0x9f => size_compound(&buffer, (buffer[0] & 0x0f) as usize, 1),
        // array(16)
        0xdc if buffer.len() < 3 => None,
        0xdc => size_compound(&buffer, get_be_size(&buffer, S16), 3),
        // array(32)
        0xdd if buffer.len() < 5 => None,
        0xdd => size_compound(&buffer, get_be_size(&buffer, S32), 5),
        //
        // *Map*
        // fixmap
        0x80..=0x8f => size_compound(&buffer, (buffer[0] & 0x0f) as usize * 2, 1),
        // array(16)
        0xde if buffer.len() < 3 => None,
        0xde => size_compound(&buffer, get_be_size(&buffer, S16) * 2, 3),
        // array(32)
        0xdf if buffer.len() < 5 => None,
        0xdf => size_compound(&buffer, get_be_size(&buffer, S32) * 2, 5),
    }
}
