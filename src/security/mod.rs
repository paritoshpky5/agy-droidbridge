use axum::http::header::{AUTHORIZATION, HOST, ORIGIN};
use axum::http::{HeaderMap, Uri, uri::Authority};
use std::net::IpAddr;
use subtle::ConstantTimeEq;

pub fn permits_origin(headers: &HeaderMap, uri: &Uri) -> bool {
    if headers.contains_key(ORIGIN) || headers.get_all(HOST).iter().count() > 1 {
        return false;
    }
    if let Some(authority) = uri.authority() {
        if !is_local(authority) {
            return false;
        }
    }
    headers.get(HOST).is_none_or(|host| {
        host.to_str()
            .ok()
            .and_then(|host| host.parse::<Authority>().ok())
            .is_some_and(|authority| is_local(&authority))
    })
}

fn is_local(authority: &Authority) -> bool {
    let host = authority.host();
    !authority.as_str().contains('@')
        && (host.eq_ignore_ascii_case("localhost")
            || host
                .trim_matches(['[', ']'])
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback()))
}

pub fn authenticates(headers: &HeaderMap, key: Option<&str>, anthropic: bool) -> bool {
    let Some(key) = key else {
        return true;
    };
    let mut bearer = headers.get_all(AUTHORIZATION).iter();
    let mut api_key = headers.get_all("x-api-key").iter();
    let token = match (bearer.next(), api_key.next()) {
        (Some(value), None) if bearer.next().is_none() => value
            .to_str()
            .ok()
            .and_then(|value| value.split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map(|(_, token)| token.as_bytes()),
        (None, Some(value)) if anthropic && api_key.next().is_none() => Some(value.as_bytes()),
        _ => None,
    };
    token.is_some_and(|value| bool::from(value.ct_eq(key.as_bytes())))
}
