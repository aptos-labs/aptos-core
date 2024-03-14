// Copyright © Aptos Foundation

use aptos_types::network_address::{parse_ip_tcp,NetworkAddress};
use std::io::Result;
use std::net::SocketAddr;
//use core::net::SocketAddr;

pub fn listen(addr: NetworkAddress) -> Result<(tokio::net::TcpListener, NetworkAddress)> {
    let ((ipaddr, port), addr_suffix) =
        parse_ip_tcp(addr.as_slice()).ok_or_else(|| invalid_addr_error(&addr))?;
    if !addr_suffix.is_empty() {
        return Err(invalid_addr_error(&addr));
    }

    let addr = SocketAddr::new(ipaddr, port);

    let socket = if ipaddr.is_ipv4() {
        tokio::net::TcpSocket::new_v4()?
    } else {
        tokio::net::TcpSocket::new_v6()?
    };

    // TODO: bring back configurable buffer sizes
    // if let Some(rx_buf) = self.tcp_buff_cfg.inbound_rx_buffer_bytes {
    //     socket.set_recv_buffer_size(rx_buf)?;
    // }
    // if let Some(tx_buf) = self.tcp_buff_cfg.inbound_tx_buffer_bytes {
    //     socket.set_send_buffer_size(tx_buf)?;
    // }
    socket.set_reuseaddr(true)?;
    socket.bind(addr)?;

    let listener = socket.listen(256)?;
    let listen_addr = NetworkAddress::from(listener.local_addr()?);

    Ok((
        listener,
        listen_addr,
    ))
}

pub fn invalid_addr_error(addr: &NetworkAddress) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("Invalid NetworkAddress: '{}'", addr),
    )
}
