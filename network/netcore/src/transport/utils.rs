// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_proxy::Proxy;
use aptos_types::network_address::{
    IpFilter, NetworkAddress, Protocol,
    Protocol::{Dns, Dns4, Dns6, Ip4, Ip6},
};
use std::{io, net::SocketAddr};
use tokio::net::lookup_host;
use url::Url;

/// Creates a proxy address string if one is required based on the given protocols
pub fn create_proxy_addr(protos: &[Protocol]) -> Option<String> {
    let proxy_addr = {
        let proxy = Proxy::new();
        let addr = match protos.first() {
            Some(Ip4(ip)) => proxy.https(&ip.to_string()),
            Some(Ip6(ip)) => proxy.https(&ip.to_string()),
            Some(Dns(name)) | Some(Dns4(name)) | Some(Dns6(name)) => proxy.https(name.as_ref()),
            _ => None,
        };

        addr.and_then(|https_proxy| Url::parse(https_proxy).ok())
            .and_then(|url| {
                if url.has_host() && url.scheme() == "http" {
                    Some(format!(
                        "{}:{}",
                        url.host().unwrap(),
                        url.port_or_known_default().unwrap()
                    ))
                } else {
                    None
                }
            })
    };
    proxy_addr
}

/// A utility function that returns an invalid address error
pub fn invalid_addr_error(addr: &NetworkAddress) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Invalid NetworkAddress: '{}'", addr),
    )
}

/// Try to lookup the dns name, then filter addrs according to the `IpFilter`
pub async fn resolve_with_filter(
    ip_filter: IpFilter,
    dns_name: &str,
    port: u16,
) -> io::Result<impl Iterator<Item = SocketAddr> + '_> {
    Ok(lookup_host((dns_name, port))
        .await?
        .filter(move |socketaddr| ip_filter.matches(socketaddr.ip())))
}
