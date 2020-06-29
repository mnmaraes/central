use failure::{format_err, Error};

use tracing::{error, info};

use serde::de::DeserializeOwned;

use serde_json as json;

pub fn parsed_encoded_error(value: &json::Value) -> Result<json::Value, Error> {
    let err = format_err!(
        "Encoding Error: Expected encoded Object, found: {:?}",
        value
    );

    let map = parse_map(value)?;
    let (_method, params) = map.iter().next().ok_or(err)?;
    let id = params["rqs_id"].clone();

    Ok(json::json!({
        "jsonrpc": "2.0",
        "error": value,
        "result": json::Value::Null,
        "id": id
    }))
}

pub fn parsed_encoded_response(value: &json::Value) -> Result<json::Value, Error> {
    let err = format_err!(
        "Encoding Error: Expected encoded Object, found: {:?}",
        value
    );

    let map = parse_map(value)?;
    let (_method, params) = map.iter().next().ok_or(err)?;
    let id = params["rqs_id"].clone();

    Ok(json::json!({
        "jsonrpc": "2.0",
        "result": value,
        "error": json::Value::Null,
        "id": id
    }))
}

pub fn parsed_encoded_notification(value: &json::Value) -> Result<json::Value, Error> {
    let err = format_err!(
        "Encoding Error: Expected encoded Object, found: {:?}",
        value
    );

    let map = parse_map(value)?;
    let (method, params) = map.iter().next().ok_or(err)?;

    Ok(json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    }))
}

pub fn parsed_encoded_request(value: &json::Value) -> Result<json::Value, Error> {
    let err = format_err!(
        "Encoding Error: Expected encoded Object, found: {:?}",
        value
    );

    let map = parse_map(value)?;
    let method = map.keys().next().ok_or(err)?;

    let mut value = value.clone();

    let mut params = value[method].take();
    let id = params["rqs_id"].take();

    Ok(json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    }))
}

pub fn parse_decoded<In: DeserializeOwned>(value: &json::Value) -> Result<In, Error> {
    let opt_id = value.get("id").map(|v| v.as_u64()).flatten();
    let opt_method = value.get("method").map(|v| v.as_str()).flatten();
    let opt_params = value.get("params").map(|v| v.as_object()).flatten();
    let opt_result = value.get("result");
    let opt_error = value.get("error");

    let parsed = match (opt_method, opt_params, opt_result, opt_error) {
        (Some(method), Some(params), None, _) => {
            let mut params = params.clone();
            if let Some(id) = opt_id {
                params.insert("rqs_id".to_string(), json::json!(id));
            }
            info!("Decoding Request: {:?}: {:?}", method, params);
            json::json!({ method: params })
        }
        (None, None, Some(res), _) if res.is_object() => {
            info!("Decoding Result: {:?}", res);
            res.clone()
        }
        (None, None, None, Some(err)) if err.is_object() => {
            info!("Decoding Error: {:?}", err);
            err.clone()
        }
        _ => {
            error!(
                "Error Decoding {:?} ({:?}, {:?}, {:?}, {:?})",
                value, opt_method, opt_params, opt_result, opt_error
            );
            return Err(format_err!(
                "Decoding Error: Unknown json-rpc format: {:?}",
                value
            ));
        }
    };

    info!("Decoding parsed: {:?}", parsed);

    Ok(json::from_value(parsed)?)
}

fn parse_map(value: &json::Value) -> Result<json::Map<String, json::Value>, Error> {
    let err = format_err!("Encoding Error: Expected encoded Map, found: {:?}", value);

    value.as_object().cloned().ok_or(err)
}
