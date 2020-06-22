use serde::de::DeserializeOwned;

use failure::{format_err, Error};

pub fn parse_encoded_response_value(value: &rmpv::Value) -> Result<rmpv::Value, Error> {
    let (_, body_val) = parse_map(value)?.remove(0);
    let fields = parse_map(&body_val)?;

    let msgid = find_rqs_id(&fields)?;

    let parsed = rmpv::Value::Array(vec![
        rmpv::Value::Integer(rmpv::Integer::from(1)),
        msgid,
        rmpv::Value::Nil,
        value.clone(),
    ]);

    Ok(parsed)
}

pub fn parse_encoded_error_value(value: &rmpv::Value) -> Result<rmpv::Value, Error> {
    let (_, body_val) = parse_map(value)?.remove(0);
    let fields = parse_map(&body_val)?;

    let msgid = find_rqs_id(&fields)?;

    let parsed = rmpv::Value::Array(vec![
        rmpv::Value::Integer(rmpv::Integer::from(1)),
        msgid,
        value.clone(),
        rmpv::Value::Nil,
    ]);

    Ok(parsed)
}

pub fn parse_encoded_notification_value(value: &rmpv::Value) -> Result<rmpv::Value, Error> {
    let (method_val, body_val) = parse_map(value)?.remove(0);
    let fields = parse_map(&body_val)?;

    let field_values: Vec<_> = fields.iter().map(|(_, value)| value.clone()).collect();

    let parsed = rmpv::Value::Array(vec![
        rmpv::Value::Integer(rmpv::Integer::from(2)),
        method_val,
        rmpv::Value::Array(field_values),
    ]);

    Ok(parsed)
}

pub fn parse_encoded_request_value(value: &rmpv::Value) -> Result<rmpv::Value, Error> {
    let (method_val, body_val) = parse_map(value)?.remove(0);
    let fields = parse_map(&body_val)?;

    let msgid = find_rqs_id(&fields)?;

    let field_values: Vec<_> = fields
        .iter()
        .filter_map(|(key, value)| match parse_string(key) {
            Ok(key) if key == "rqs_id" => None,
            _ => Some(value),
        })
        .cloned()
        .collect();

    let parsed = rmpv::Value::Array(vec![
        rmpv::Value::Integer(rmpv::Integer::from(0)),
        msgid,
        method_val,
        rmpv::Value::Array(field_values),
    ]);

    Ok(parsed)
}

pub fn parse_decoded_request_value<Parsed: DeserializeOwned>(
    values: &[rmpv::Value],
) -> Result<Parsed, Error> {
    let [msgid, method, params] = match values {
        [_, msgid, method, params] => [msgid, method, params],
        _ => {
            return Err(format_err!(
                "Decode Error: Unrecognized request pattern: {:?}",
                values
            ))
        }
    };

    let mut params = parse_array(&params)?;
    params.insert(0, msgid.clone());

    let map = rmpv::Value::Map(vec![(method.clone(), rmpv::Value::Array(params))]);

    let mut buffer = vec![];
    rmpv::encode::write_value(&mut buffer, &map)?;

    Ok(rmp_serde::decode::from_slice(&buffer)?)
}

pub fn parse_decoded_notification_value<Parsed: DeserializeOwned>(
    values: &[rmpv::Value],
) -> Result<Parsed, Error> {
    let [method, params] = match values {
        [_, method, params] => [method, params],
        _ => {
            return Err(format_err!(
                "Decode Error: Unrecognized notification pattern: {:?}",
                values
            ))
        }
    };

    let map = rmpv::Value::Map(vec![(method.clone(), params.clone())]);

    let mut buffer = vec![];
    rmpv::encode::write_value(&mut buffer, &map)?;

    Ok(rmp_serde::decode::from_slice(&buffer)?)
}

pub fn parse_decoded_result_value<Parsed: DeserializeOwned>(
    values: &[rmpv::Value],
) -> Result<Parsed, Error> {
    let to_decode = match values {
        [_, _, err, params] if err.is_nil() => params,
        [_, _, err, _] => err,
        _ => {
            return Err(format_err!(
                "Decode Error: Unrecognized result pattern: {:?}",
                values
            ))
        }
    };

    let mut buffer = vec![];
    rmpv::encode::write_value(&mut buffer, &to_decode)?;

    Ok(rmp_serde::decode::from_slice(&buffer)?)
}

fn find_rqs_id(map: &[(rmpv::Value, rmpv::Value)]) -> Result<rmpv::Value, Error> {
    let err = format_err!(
        "Encoding Error: Expected to find `rqs_id` value in: {:?}",
        map
    );

    // This is a search, but the id should always be the first value
    map.iter()
        .find_map(|(key, value)| match parse_string(key) {
            Ok(key) if key == "rqs_id" => Some(value.clone()),
            _ => None,
        })
        .ok_or(err)
}

fn parse_map(value: &rmpv::Value) -> Result<Vec<(rmpv::Value, rmpv::Value)>, Error> {
    let err = format_err!("Encoding Error: Expected encoded Map, found: {:?}", value);

    value.as_map().cloned().ok_or(err)
}

pub fn parse_array(value: &rmpv::Value) -> Result<Vec<rmpv::Value>, Error> {
    let err = format_err!("Encoding Error: Expected encoded Array, found: {:?}", value);

    value.as_array().cloned().ok_or(err)
}

fn parse_string(value: &rmpv::Value) -> Result<&str, Error> {
    let err = format_err!(
        "Encoding Error: Expected encoded String, found: {:?}",
        value
    );

    value.as_str().ok_or(err)
}

pub fn parse_int(value: &rmpv::Value) -> Result<u64, Error> {
    let err = format_err!(
        "Encoding Error: Expected encoded unsigned integer, found: {:?}",
        value
    );

    value.as_u64().ok_or(err)
}
