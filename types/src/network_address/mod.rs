// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use aptos_crypto::{
    traits::{CryptoMaterialError, ValidCryptoMaterialStringExt},
    x25519,
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    convert::{Into, TryFrom},
    fmt,
    iter::IntoIterator,
    net::{self, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    num,
    str::FromStr,
    string::ToString,
};
use thiserror::Error;

const MAX_DNS_NAME_SIZE: usize = 255;

/// ## Overview
///
/// Aptos `NetworkAddress` is a compact, efficient, self-describing and
/// future-proof network address represented as a stack of protocols. Essentially
/// libp2p's [multiaddr] but using [`bcs`] to describe the binary format.
///
/// Most validators will advertise a network address like:
///
/// `/dns/example.com/tcp/6180/noise-ik/<x25519-pubkey>/handshake/1`
///
/// Unpacking, the above effectively means:
///
/// 1. Resolve the DNS name "example.com" to an ip address, `addr`.
/// 2. Open a TCP connection to `(addr, 6180)`.
/// 3. Perform a Noise IK handshake and assume the peer's static pubkey is
///    `<x25519-pubkey>`. After this step, we will have a secure, authenticated
///    connection with the peer.
/// 4. Perform a AptosNet version negotiation handshake (version 1).
///
/// ## Self-describing, Upgradable
///
/// One key concept behind `NetworkAddress` is that it is fully self-describing,
/// which allows us to easily "pre-negotiate" protocols while also allowing for
/// future upgrades. For example, it is generally unsafe to negotiate a secure
/// transport in-band. Instead, with `NetworkAddress` we can advertise (via
/// discovery) the specific secure transport protocol and public key that we
/// support (and even advertise multiple incompatible versions). When a peer
/// wishes to establish a connection with us, they already know which secure
/// transport protocol to use; in this sense, the secure transport protocol is
/// "pre-negotiated" by the dialier selecting which advertised protocol to use.
///
/// Each network address is encoded with the length of the encoded `NetworkAddress`
/// and then the serialized protocol slices to allow for transparent upgradeability.
/// For example, if the current software cannot decode a `NetworkAddress` within
/// a `Vec<NetworkAddress>` it can still decode the underlying `Vec<u8>` and
/// retrieve the remaining `Vec<NetworkAddress>`.
///
/// ## Transport
///
/// In addition, `NetworkAddress` is integrated with the AptosNet concept of a
/// [`Transport`], which takes a `NetworkAddress` when dialing and peels off
/// [`Protocol`]s to establish a connection and perform initial handshakes.
/// Similarly, the [`Transport`] takes `NetworkAddress` to listen on, which tells
/// it what protocols to expect on the socket.
///
/// ## Example
///
/// An example of a serialized `NetworkAddress`:
///
/// ```rust
/// // human-readable format:
/// //
/// //   "/ip4/10.0.0.16/tcp/80"
/// //
/// // serialized NetworkAddress:
/// //
/// //      [ 09 02 00 0a 00 00 10 05 80 00 ]
/// //          \  \  \  \           \  \
/// //           \  \  \  \           \  '-- u16 tcp port
/// //            \  \  \  \           '-- uvarint protocol id for /tcp
/// //             \  \  \  '-- u32 ipv4 address
/// //              \  \  '-- uvarint protocol id for /ip4
/// //               \  '-- uvarint number of protocols
/// //                '-- length of encoded network address
///
/// use aptos_types::network_address::NetworkAddress;
/// use bcs;
/// use std::{str::FromStr, convert::TryFrom};
///
/// let addr = NetworkAddress::from_str("/ip4/10.0.0.16/tcp/80").unwrap();
/// let actual_ser_addr = bcs::to_bytes(&addr).unwrap();
///
/// let expected_ser_addr: Vec<u8> = [9, 2, 0, 10, 0, 0, 16, 5, 80, 0].to_vec();
///
/// assert_eq!(expected_ser_addr, actual_ser_addr);
/// ```
///
/// [multiaddr]: https://multiformats.io/multiaddr/
/// [`Transport`]: ../netcore/transport/trait.Transport.html
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct NetworkAddress(Vec<Protocol>);

/// A single protocol in the [`NetworkAddress`] protocol stack.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum Protocol {
    Ip4(Ipv4Addr),
    Ip6(Ipv6Addr),
    Dns(DnsName),
    Dns4(DnsName),
    Dns6(DnsName),
    Tcp(u16),
    Memory(u16),
    // human-readable x25519::PublicKey is lower-case hex encoded
    NoiseIK(x25519::PublicKey),
    // TODO(philiphayes): use actual handshake::MessagingProtocolVersion. we
    // probably need to move network wire into its own crate to avoid circular
    // dependency b/w network and types.
    Handshake(u8),
}

/// A minimally parsed DNS name. We don't really do any checking other than
/// enforcing:
///
/// 1. it is not an empty string
/// 2. it is not larger than 255 bytes
/// 3. it does not contain any forward slash '/' characters
/// 4. it is valid ASCII (Unicode characters are not allowed to prevent phishing attacks)
///
/// From the [DNS name syntax RFC](https://tools.ietf.org/html/rfc2181#page-13),
/// the standard rules are:
///
/// 1. the total size <= 255 bytes
/// 2. each label <= 63 bytes
/// 3. any binary string is valid
///
/// So the restrictions we're adding are (1) no '/' characters and (2) the name
/// is a valid ASCII string. We do this because '/' characters are already our
/// protocol delimiter and Rust's [`std::net::ToSocketAddrs`] API requires a
/// `&str`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct DnsName(String);

/// Possible errors when parsing a human-readable [`NetworkAddress`].
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unknown protocol type: '{0}'")]
    UnknownProtocolType(String),

    #[error("unexpected end of protocol string")]
    UnexpectedEnd,

    #[error("error parsing ip4/ip6 address: {0}")]
    ParseAddrError(#[from] net::AddrParseError),

    #[error("error parsing integer: {0}")]
    ParseIntError(#[from] num::ParseIntError),

    #[error("error parsing x25519 public key: {0}")]
    ParseX25519PubkeyError(#[from] CryptoMaterialError),

    #[error("network address cannot be empty")]
    EmptyProtocolString,

    #[error("protocol string must start with '/'")]
    InvalidProtocolString,

    #[error("dns name cannot be empty")]
    EmptyDnsNameString,

    #[error("dns name cannot contain '/' characters")]
    InvalidDnsNameCharacter,

    #[error("dns name contains non-ASCII characters: {0}")]
    DnsNameNonASCII(String),

    #[error("dns name is too long: len: {0} bytes, max len: 255 bytes")]
    DnsNameTooLong(usize),

    #[error("error decrypting network address")]
    DecryptError,

    #[error("bcs error: {0}")]
    BCSError(#[from] bcs::Error),

    #[error("NetworkAddress must start with one of Protocol::Ip4/Ip6/Dns/Dns4/Dns6")]
    NetworkLayerMissing,

    #[error(
        "NetworkAddress must start with one of Protocol::Ip4/Ip6/Dns/Dns4/Dns6 followed by TCP"
    )]
    TransportLayerMissing,

    #[error("NetworkAddress must have a NoiseIK protocol following the TCP protocol")]
    SessionLayerMissing,

    #[error("NetworkAddress must have a Handshake protocol following the NoiseIK protocol")]
    HandshakeLayerMissing,

    #[error("NetworkAddress must not have duplicate layer protocols")]
    RedundantLayer,
}

#[derive(Error, Debug)]
#[error("network address cannot be empty")]
pub struct EmptyError;

////////////////////
// NetworkAddress //
////////////////////
// Network
// Transport
// Session - Noise
// Presentation - Handshake

fn is_network_layer(p: Option<&Protocol>) -> bool {
    use Protocol::*;

    matches!(
        p,
        Some(Ip4(_))
            | Some(Ip6(_))
            | Some(Dns(_))
            | Some(Dns4(_))
            | Some(Dns6(_))
            /* cfg!(test) is the correct value rather than true, but this doesnt propagate properly from external tests */
            | Some(Memory(_))
    )
}

fn is_transport_layer(p: Option<&Protocol>) -> bool {
    use Protocol::*;

    matches!(p, Some(Tcp(_)))
}

fn is_session_layer(p: Option<&Protocol>, allow_empty: bool) -> bool {
    use Protocol::*;
    match p {
        None => allow_empty,
        Some(NoiseIK(_)) => true,
        _ => false,
    }
}

fn is_handshake_layer(p: Option<&Protocol>, allow_empty: bool) -> bool {
    use Protocol::*;
    match p {
        None => allow_empty,
        Some(Handshake(_)) => true,
        _ => false,
    }
}

impl NetworkAddress {
    pub fn from_protocols(protocols: Vec<Protocol>) -> Result<Self, ParseError> {
        use Protocol::*;

        let mut iter = protocols.iter();

        let mut p = iter.next();

        if p.is_none() {
            return Ok(Self(protocols));
        }

        if !is_network_layer(p) {
            return Err(ParseError::NetworkLayerMissing);
        }

        if !matches!(p, Some(Memory(_))) {
            p = iter.next();
            if p.is_none() {
                return Ok(Self(protocols));
            }
            if !is_transport_layer(p) {
                return Err(ParseError::TransportLayerMissing);
            }
        }

        p = iter.next();
        if p.is_none() {
            return Ok(Self(protocols));
        }
        if !is_session_layer(p, true) {
            return Err(ParseError::SessionLayerMissing);
        }

        p = iter.next();
        if p.is_none() {
            return Ok(Self(protocols));
        }
        if !is_handshake_layer(p, true) {
            return Err(ParseError::HandshakeLayerMissing);
        }

        p = iter.next();
        if p.is_none() {
            Ok(Self(protocols))
        } else {
            Err(ParseError::RedundantLayer)
        }
    }

    pub fn as_slice(&self) -> &[Protocol] {
        self.0.as_slice()
    }

    fn push(mut self, proto: Protocol) -> Self {
        self.0.push(proto);
        self
    }

    /// Given a base `NetworkAddress`, append production protocols and
    /// return the modified `NetworkAddress`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use aptos_crypto::{traits::ValidCryptoMaterialStringExt, x25519};
    /// use aptos_types::network_address::NetworkAddress;
    /// use std::str::FromStr;
    ///
    /// let pubkey_str = "080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120";
    /// let pubkey = x25519::PublicKey::from_encoded_string(pubkey_str).unwrap();
    /// let addr = NetworkAddress::from_str("/dns/example.com/tcp/6180").unwrap();
    /// let addr = addr.append_prod_protos(pubkey, 0);
    /// assert_eq!(
    ///     addr.to_string(),
    ///     "/dns/example.com/tcp/6180/noise-ik/0x080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120/handshake/0",
    /// );
    /// ```
    // TODO(philiphayes): use handshake version enum
    pub fn append_prod_protos(
        self,
        network_pubkey: x25519::PublicKey,
        handshake_version: u8,
    ) -> Self {
        self.push(Protocol::NoiseIK(network_pubkey))
            .push(Protocol::Handshake(handshake_version))
    }

    /// Check that a `NetworkAddress` looks like a typical AptosNet address with
    /// associated protocols.
    ///
    /// "typical" AptosNet addresses begin with a transport protocol:
    ///
    /// `"/ip4/<addr>/tcp/<port>"` or
    /// `"/ip6/<addr>/tcp/<port>"` or
    /// `"/dns4/<domain>/tcp/<port>"` or
    /// `"/dns6/<domain>/tcp/<port>"` or
    /// `"/dns/<domain>/tcp/<port>"` or
    /// cfg!(test) `"/memory/<port>"`
    ///
    /// followed by transport upgrade handshake protocols:
    ///
    /// `"/noise-ik/<pubkey>/handshake/<version>"`
    ///
    /// ### Example
    ///
    /// ```rust
    /// use aptos_types::network_address::NetworkAddress;
    /// use std::str::FromStr;
    ///
    /// let addr_str = "/ip4/1.2.3.4/tcp/6180/noise-ik/080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120/handshake/0";
    /// let addr = NetworkAddress::from_str(addr_str).unwrap();
    /// assert!(addr.is_aptosnet_addr());
    /// ```
    pub fn is_aptosnet_addr(&self) -> bool {
        parse_aptosnet_protos(self.as_slice()).is_some()
    }

    /// Retrieves the IP address from the network address
    pub fn find_ip_addr(&self) -> Option<IpAddr> {
        self.0.iter().find_map(|proto| match proto {
            Protocol::Ip4(addr) => Some(IpAddr::V4(*addr)),
            Protocol::Ip6(addr) => Some(IpAddr::V6(*addr)),
            _ => None,
        })
    }

    pub fn find_dns_name(&self) -> Option<DnsName> {
        self.0.iter().find_map(|proto| match proto {
            Protocol::Dns(addr) => Some(addr.clone()),
            Protocol::Dns4(addr) => Some(addr.clone()),
            Protocol::Dns6(addr) => Some(addr.clone()),
            _ => None,
        })
    }

    /// Retrieves the port from the network address
    pub fn find_port(&self) -> Option<u16> {
        self.0.iter().find_map(|proto| match proto {
            Protocol::Tcp(port) => Some(*port),
            _ => None,
        })
    }

    /// A temporary, hacky function to parse out the first `/noise-ik/<pubkey>` from
    /// a `NetworkAddress`. We can remove this soon, when we move to the interim
    /// "monolithic" transport model.
    pub fn find_noise_proto(&self) -> Option<x25519::PublicKey> {
        self.0.iter().find_map(|proto| match proto {
            Protocol::NoiseIK(pubkey) => Some(*pubkey),
            _ => None,
        })
    }

    /// A function to rotate public keys for `NoiseIK` protocols
    pub fn rotate_noise_public_key(
        &mut self,
        to_replace: &x25519::PublicKey,
        new_public_key: &x25519::PublicKey,
    ) {
        for protocol in self.0.iter_mut() {
            // Replace the public key in any Noise protocols that match the key
            if let Protocol::NoiseIK(public_key) = protocol {
                if public_key == to_replace {
                    *protocol = Protocol::NoiseIK(*new_public_key);
                }
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn mock() -> Self {
        NetworkAddress::from_protocols(vec![Protocol::Memory(1234)]).unwrap()
    }
}

impl IntoIterator for NetworkAddress {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = Protocol;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromStr for NetworkAddress {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::EmptyProtocolString);
        }

        let mut protocols = Vec::new();
        let mut parts_iter = s.split('/');

        // the first character must be '/'
        if parts_iter.next() != Some("") {
            return Err(ParseError::InvalidProtocolString);
        }

        // parse all `Protocol`s
        while let Some(protocol_type) = parts_iter.next() {
            // Allow for trailing or duplicate '/'
            if protocol_type.is_empty() {
                continue;
            }
            protocols.push(Protocol::parse(protocol_type, &mut parts_iter)?);
        }
        if protocols.is_empty() {
            Err(ParseError::EmptyProtocolString)
        } else {
            NetworkAddress::from_protocols(protocols)
        }
    }
}

impl ToSocketAddrs for NetworkAddress {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> Result<Self::Iter, std::io::Error> {
        if let Some(((ipaddr, port), _)) = parse_ip_tcp(self.as_slice()) {
            Ok(vec![SocketAddr::new(ipaddr, port)].into_iter())
        } else if let Some(((ip_filter, dns_name, port), _)) = parse_dns_tcp(self.as_slice()) {
            format!("{}:{}", dns_name, port).to_socket_addrs().map(|v| {
                v.filter(|addr| ip_filter.matches(addr.ip()))
                    .collect::<Vec<_>>()
                    .into_iter()
            })
        } else {
            Ok(vec![].into_iter())
        }
    }
}

impl TryFrom<Vec<Protocol>> for NetworkAddress {
    type Error = ParseError;

    fn try_from(value: Vec<Protocol>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ParseError::EmptyProtocolString)
        } else {
            NetworkAddress::from_protocols(value)
        }
    }
}

impl From<Protocol> for NetworkAddress {
    fn from(proto: Protocol) -> NetworkAddress {
        NetworkAddress::from_protocols(vec![proto]).unwrap()
    }
}

impl From<SocketAddr> for NetworkAddress {
    fn from(sockaddr: SocketAddr) -> NetworkAddress {
        let ip_proto = Protocol::from(sockaddr.ip());
        let tcp_proto = Protocol::Tcp(sockaddr.port());
        NetworkAddress::from_protocols(vec![ip_proto, tcp_proto]).unwrap()
    }
}

impl fmt::Display for NetworkAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for protocol in self.0.iter() {
            protocol.fmt(f)?;
        }
        Ok(())
    }
}

impl fmt::Debug for NetworkAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for NetworkAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            #[derive(Serialize)]
            #[serde(rename = "NetworkAddress")]
            struct Wrapper<'a>(#[serde(with = "serde_bytes")] &'a [u8]);

            bcs::to_bytes(&self.as_slice())
                .map_err(serde::ser::Error::custom)
                .and_then(|v| Wrapper(&v).serialize(serializer))
        }
    }
}

impl<'de> Deserialize<'de> for NetworkAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = <String>::deserialize(deserializer)?;
            NetworkAddress::from_str(s.as_str()).map_err(de::Error::custom)
        } else {
            #[derive(Deserialize)]
            #[serde(rename = "NetworkAddress")]
            struct Wrapper(#[serde(with = "serde_bytes")] Vec<u8>);

            Wrapper::deserialize(deserializer)
                .and_then(|v| bcs::from_bytes(&v.0).map_err(de::Error::custom))
                .and_then(|v: Vec<Protocol>| NetworkAddress::try_from(v).map_err(de::Error::custom))
        }
    }
}

#[cfg(any(test, feature = "fuzzing"))]
pub fn arb_aptosnet_addr() -> impl Strategy<Value = NetworkAddress> {
    let arb_transport_protos = prop_oneof![
        any::<u16>().prop_map(|port| vec![Protocol::Memory(port)]),
        any::<(Ipv4Addr, u16)>()
            .prop_map(|(addr, port)| vec![Protocol::Ip4(addr), Protocol::Tcp(port)]),
        any::<(Ipv6Addr, u16)>()
            .prop_map(|(addr, port)| vec![Protocol::Ip6(addr), Protocol::Tcp(port)]),
        any::<(DnsName, u16)>()
            .prop_map(|(name, port)| vec![Protocol::Dns(name), Protocol::Tcp(port)]),
        any::<(DnsName, u16)>()
            .prop_map(|(name, port)| vec![Protocol::Dns4(name), Protocol::Tcp(port)]),
        any::<(DnsName, u16)>()
            .prop_map(|(name, port)| vec![Protocol::Dns6(name), Protocol::Tcp(port)]),
    ];
    let arb_aptosnet_protos = any::<(x25519::PublicKey, u8)>()
        .prop_map(|(pubkey, hs)| vec![Protocol::NoiseIK(pubkey), Protocol::Handshake(hs)]);

    (arb_transport_protos, arb_aptosnet_protos).prop_map(
        |(mut transport_protos, mut aptosnet_protos)| {
            transport_protos.append(&mut aptosnet_protos);
            NetworkAddress::from_protocols(transport_protos).unwrap()
        },
    )
}

//////////////
// Protocol //
//////////////

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Protocol::*;
        match self {
            Ip4(addr) => write!(f, "/ip4/{}", addr),
            Ip6(addr) => write!(f, "/ip6/{}", addr),
            Dns(domain) => write!(f, "/dns/{}", domain),
            Dns4(domain) => write!(f, "/dns4/{}", domain),
            Dns6(domain) => write!(f, "/dns6/{}", domain),
            Tcp(port) => write!(f, "/tcp/{}", port),
            Memory(port) => write!(f, "/memory/{}", port),
            NoiseIK(pubkey) => write!(
                f,
                "/noise-ik/{}",
                pubkey
                    .to_encoded_string()
                    .expect("ValidCryptoMaterialStringExt::to_encoded_string is infallible")
            ),
            Handshake(version) => write!(f, "/handshake/{}", version),
        }
    }
}

fn parse_one<'a, T>(args: &mut impl Iterator<Item = &'a str>) -> Result<T, ParseError>
where
    T: FromStr,
    T::Err: Into<ParseError>,
{
    let next_arg = args.next().ok_or(ParseError::UnexpectedEnd)?;
    next_arg.parse().map_err(Into::into)
}

impl Protocol {
    fn parse<'a>(
        protocol_type: &str,
        args: &mut impl Iterator<Item = &'a str>,
    ) -> Result<Protocol, ParseError> {
        let protocol = match protocol_type {
            "ip4" => Protocol::Ip4(parse_one(args)?),
            "ip6" => Protocol::Ip6(parse_one(args)?),
            "dns" => Protocol::Dns(parse_one(args)?),
            "dns4" => Protocol::Dns4(parse_one(args)?),
            "dns6" => Protocol::Dns6(parse_one(args)?),
            "tcp" => Protocol::Tcp(parse_one(args)?),
            "memory" => Protocol::Memory(parse_one(args)?),
            "noise-ik" => Protocol::NoiseIK(x25519::PublicKey::from_encoded_string(
                args.next().ok_or(ParseError::UnexpectedEnd)?,
            )?),
            "handshake" => Protocol::Handshake(parse_one(args)?),
            unknown => return Err(ParseError::UnknownProtocolType(unknown.to_string())),
        };
        Ok(protocol)
    }
}

impl From<IpAddr> for Protocol {
    fn from(addr: IpAddr) -> Protocol {
        match addr {
            IpAddr::V4(addr) => Protocol::Ip4(addr),
            IpAddr::V6(addr) => Protocol::Ip6(addr),
        }
    }
}

/////////////
// DnsName //
/////////////

impl DnsName {
    fn validate(s: &str) -> Result<(), ParseError> {
        if s.is_empty() {
            Err(ParseError::EmptyDnsNameString)
        } else if s.as_bytes().len() > MAX_DNS_NAME_SIZE {
            Err(ParseError::DnsNameTooLong(s.as_bytes().len()))
        } else if s.contains('/') {
            Err(ParseError::InvalidDnsNameCharacter)
        } else if !s.is_ascii() {
            Err(ParseError::DnsNameNonASCII(s.into()))
        } else {
            Ok(())
        }
    }
}

impl From<DnsName> for String {
    fn from(dns_name: DnsName) -> String {
        dns_name.0
    }
}

impl AsRef<str> for DnsName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl TryFrom<String> for DnsName {
    type Error = ParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        DnsName::validate(s.as_str()).map(|_| DnsName(s))
    }
}

impl FromStr for DnsName {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DnsName::validate(s).map(|_| DnsName(s.to_owned()))
    }
}

impl fmt::Display for DnsName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for DnsName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "DnsName")]
        struct DeserializeWrapper(String);

        let wrapper = DeserializeWrapper::deserialize(deserializer)?;
        let name = DnsName::try_from(wrapper.0).map_err(de::Error::custom)?;
        Ok(name)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for DnsName {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        // generate arbitrary unicode strings
        // + without '/'
        // + without control characters (so we can print them easily)
        // + between 1-255 bytes in length
        r"[^/\pC]{1,255}"
            // need this filter b/c the number of unicode characters does not
            // necessarily equal the number of bytes.
            .prop_filter_map("string too long", |s| {
                if s.as_bytes().len() > MAX_DNS_NAME_SIZE {
                    None
                } else {
                    Some(DnsName(s))
                }
            })
            .boxed()
    }
}

/////////////
// Parsing //
/////////////

/// parse the `&[Protocol]` into the `"/memory/<port>"` prefix and unparsed
/// `&[Protocol]` suffix.
pub fn parse_memory(protos: &[Protocol]) -> Option<(u16, &[Protocol])> {
    protos
        .split_first()
        .and_then(|(first, suffix)| match first {
            Protocol::Memory(port) => Some((*port, suffix)),
            _ => None,
        })
}

/// parse the `&[Protocol]` into the `"/ip4/<addr>/tcp/<port>"` or
/// `"/ip6/<addr>/tcp/<port>"` prefix and unparsed `&[Protocol]` suffix.
pub fn parse_ip_tcp(protos: &[Protocol]) -> Option<((IpAddr, u16), &[Protocol])> {
    use Protocol::*;

    if protos.len() < 2 {
        return None;
    }

    let (prefix, suffix) = protos.split_at(2);
    match prefix {
        [Ip4(ip), Tcp(port)] => Some(((IpAddr::V4(*ip), *port), suffix)),
        [Ip6(ip), Tcp(port)] => Some(((IpAddr::V6(*ip), *port), suffix)),
        _ => None,
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IpFilter {
    Any,
    OnlyIp4,
    OnlyIp6,
}

impl IpFilter {
    pub fn matches(&self, ipaddr: IpAddr) -> bool {
        match self {
            IpFilter::Any => true,
            IpFilter::OnlyIp4 => ipaddr.is_ipv4(),
            IpFilter::OnlyIp6 => ipaddr.is_ipv6(),
        }
    }
}

/// parse the `&[Protocol]` into the `"/dns/<domain>/tcp/<port>"`,
/// `"/dns4/<domain>/tcp/<port>"`, or `"/dns6/<domain>/tcp/<port>"` prefix and
/// unparsed `&[Protocol]` suffix.
pub fn parse_dns_tcp(protos: &[Protocol]) -> Option<((IpFilter, &DnsName, u16), &[Protocol])> {
    use Protocol::*;

    if protos.len() < 2 {
        return None;
    }

    let (prefix, suffix) = protos.split_at(2);
    match prefix {
        [Dns(name), Tcp(port)] => Some(((IpFilter::Any, name, *port), suffix)),
        [Dns4(name), Tcp(port)] => Some(((IpFilter::OnlyIp4, name, *port), suffix)),
        [Dns6(name), Tcp(port)] => Some(((IpFilter::OnlyIp6, name, *port), suffix)),
        _ => None,
    }
}

pub fn parse_tcp(protos: &[Protocol]) -> Option<((String, u16), &[Protocol])> {
    use Protocol::*;

    if protos.len() < 2 {
        return None;
    }

    let (prefix, suffix) = protos.split_at(2);
    match prefix {
        [Ip4(ip), Tcp(port)] => Some(((ip.to_string(), *port), suffix)),
        [Ip6(ip), Tcp(port)] => Some(((ip.to_string(), *port), suffix)),
        [Dns(name), Tcp(port)] => Some(((name.to_string(), *port), suffix)),
        [Dns4(name), Tcp(port)] => Some(((name.to_string(), *port), suffix)),
        [Dns6(name), Tcp(port)] => Some(((name.to_string(), *port), suffix)),
        _ => None,
    }
}

/// parse the `&[Protocol]` into the `"/noise-ik/<pubkey>"` prefix and
/// unparsed `&[Protocol]` suffix.
pub fn parse_noise_ik(protos: &[Protocol]) -> Option<(&x25519::PublicKey, &[Protocol])> {
    match protos.split_first() {
        Some((Protocol::NoiseIK(pubkey), suffix)) => Some((pubkey, suffix)),
        _ => None,
    }
}

/// parse the `&[Protocol]` into the `"/handshake/<version>"` prefix and
/// unparsed `&[Protocol]` suffix.
pub fn parse_handshake(protos: &[Protocol]) -> Option<u8> {
    match protos.last() {
        Some(Protocol::Handshake(version)) => Some(*version),
        _ => None,
    }
}

/// parse canonical aptosnet protocols
///
/// See: [`NetworkAddress::is_aptosnet_addr`]
fn parse_aptosnet_protos(protos: &[Protocol]) -> Option<&[Protocol]> {
    // parse base transport layer
    // ---
    // parse_ip_tcp
    // <or> parse_dns_tcp
    // <or> cfg!(test) parse_memory

    let transport_suffix = parse_ip_tcp(protos)
        .map(|x| x.1)
        .or_else(|| parse_dns_tcp(protos).map(|x| x.1))
        .or_else(|| {
            if cfg!(test) {
                parse_memory(protos).map(|x| x.1)
            } else {
                None
            }
        })?;

    // parse authentication layer
    // ---
    // parse_noise_ik

    let auth_suffix = parse_noise_ik(transport_suffix).map(|x| x.1)?;

    // parse handshake layer

    // also ensures there are no trailing protos after handshake
    parse_handshake(auth_suffix)?;

    Some(protos)
}

///////////
// Tests //
///////////

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::format_err;
    use bcs::test_helpers::assert_canonical_encode_decode;
    use claims::assert_matches;

    #[test]
    fn test_network_address_display() {
        use super::Protocol::*;
        let pubkey_str = "080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120";
        let pubkey = x25519::PublicKey::from_encoded_string(pubkey_str).unwrap();
        let protocols = vec![
            Dns(DnsName("example.com".to_owned())),
            Tcp(1234),
            NoiseIK(pubkey),
            Handshake(0),
        ];

        let addr = NetworkAddress::from_protocols(protocols).unwrap();

        let noise_addr_str = format!(
            "/dns/example.com/tcp/1234/noise-ik/0x{}/handshake/0",
            pubkey_str
        );
        assert_eq!(noise_addr_str, addr.to_string());
    }

    #[test]
    fn test_network_address_parse_success() {
        use super::Protocol::*;

        let pubkey_str = "080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120";
        let pubkey = x25519::PublicKey::from_encoded_string(pubkey_str).unwrap();
        let noise_addr_str = format!(
            "/dns/example.com/tcp/1234/noise-ik/{}/handshake/5",
            pubkey_str
        );

        let test_cases = [
            ("/memory/1234", vec![Memory(1234)]),
            (
                &(format!(
                    "/ip4/12.34.56.78/tcp/1234/noise-ik/{}/handshake/123",
                    pubkey_str
                )),
                vec![
                    Ip4(Ipv4Addr::new(12, 34, 56, 78)),
                    Tcp(1234),
                    NoiseIK(pubkey),
                    Handshake(123),
                ],
            ),
            ("/ip6/::1/tcp/0", vec![
                Ip6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                Tcp(0),
            ]),
            ("/ip6/dead:beef::c0de/tcp/8080", vec![
                Ip6(Ipv6Addr::new(0xDEAD, 0xBEEF, 0, 0, 0, 0, 0, 0xC0DE)),
                Tcp(8080),
            ]),
            ("/dns/example.com/tcp/80", vec![
                Dns(DnsName("example.com".to_owned())),
                Tcp(80),
            ]),
            (&noise_addr_str, vec![
                Dns(DnsName("example.com".to_owned())),
                Tcp(1234),
                NoiseIK(pubkey),
                Handshake(5),
            ]),
        ];

        for (addr_str, expected_address) in &test_cases {
            let actual_address = NetworkAddress::from_str(addr_str)
                .map_err(|err| format_err!("failed to parse: input: '{}', err: {}", addr_str, err))
                .unwrap();
            let expected_address =
                NetworkAddress::from_protocols(expected_address.clone()).unwrap();
            assert_eq!(actual_address, expected_address);
        }
    }

    #[test]
    fn test_network_address_parse_fail() {
        let test_cases = [
            "",
            "/",
            "/foobar",
            "/tcp",
            "tcp/1234",
            "/tcp/1234/",
            "/tcp/1234/foobar/5",
            "/tcp/99999",
            "/ip4/1.1.1",
            "/ip4/1.1.1.1.",
            "/ip4/1.1.1.1.1",
            "/ip4/1.1.1.999.1",
        ];

        for &addr_str in &test_cases {
            if let Ok(addr) = NetworkAddress::from_str(addr_str) {
                panic!(
                    "parsing should fail: input: '{}', output: '{}'",
                    addr_str, addr
                );
            }
        }
    }

    #[test]
    fn test_parse_memory() {
        let addr = NetworkAddress::from_str("/memory/123").unwrap();
        let expected_suffix: &[Protocol] = &[];
        assert_eq!(
            parse_memory(addr.as_slice()).unwrap(),
            (123, expected_suffix)
        );
    }

    #[test]
    fn test_parse_ip_tcp() {
        let addr = NetworkAddress::from_str("/ip4/1.2.3.4/tcp/123").unwrap();
        let expected_suffix: &[Protocol] = &[];
        assert_eq!(
            parse_ip_tcp(addr.as_slice()).unwrap(),
            ((IpAddr::from_str("1.2.3.4").unwrap(), 123), expected_suffix)
        );

        let addr = NetworkAddress::from_str("/ip6/::1/tcp/123").unwrap();
        let expected_suffix: &[Protocol] = &[];
        assert_eq!(
            parse_ip_tcp(addr.as_slice()).unwrap(),
            ((IpAddr::from_str("::1").unwrap(), 123), expected_suffix)
        );
    }

    #[test]
    fn test_parse_dns_tcp() {
        let dns_name = DnsName::from_str("example.com").unwrap();
        let addr = NetworkAddress::from_str("/dns/example.com/tcp/123").unwrap();
        let expected_suffix: &[Protocol] = &[];
        assert_eq!(
            parse_dns_tcp(addr.as_slice()).unwrap(),
            ((IpFilter::Any, &dns_name, 123), expected_suffix)
        );

        let addr = NetworkAddress::from_str("/dns4/example.com/tcp/123").unwrap();
        let expected_suffix: &[Protocol] = &[];
        assert_eq!(
            parse_dns_tcp(addr.as_slice()).unwrap(),
            ((IpFilter::OnlyIp4, &dns_name, 123), expected_suffix)
        );

        let addr = NetworkAddress::from_str("/dns6/example.com/tcp/123").unwrap();
        let expected_suffix: &[Protocol] = &[];
        assert_eq!(
            parse_dns_tcp(addr.as_slice()).unwrap(),
            ((IpFilter::OnlyIp6, &dns_name, 123), expected_suffix)
        );

        // The first `e` in `example.com` is a unicode character and not a regular `e`!
        let bad_address = "/dns6/еxample.com/tcp/123";
        assert_matches!(
            NetworkAddress::from_str(bad_address),
            Err(ParseError::DnsNameNonASCII(_))
        );
    }

    #[test]
    fn test_find_noise_proto() {
        let pubkey_str = "080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120";
        let pubkey = x25519::PublicKey::from_encoded_string(pubkey_str).unwrap();
        let addr = NetworkAddress::from_str(&format!(
            "/dns4/example.com/tcp/1024/noise-ik/{}/handshake/0",
            pubkey_str
        ))
        .unwrap();

        assert_eq!(addr.find_noise_proto().unwrap(), pubkey);

        let addr = NetworkAddress::from_str(&format!(
            "/dns4/example.com/tcp/999/noise-ik/{}/handshake/0",
            pubkey_str
        ))
        .unwrap();
        assert_eq!(addr.find_noise_proto().unwrap(), pubkey);
    }

    #[test]
    fn test_parse_handshake() {
        let pubkey_str = "080e287879c918794170e258bfaddd75acac5b3e350419044655e4983a487120";
        let addr = NetworkAddress::from_str(&format!(
            "/dns4/example.com/tcp/999/noise-ik/{}/handshake/0",
            pubkey_str
        ))
        .unwrap();

        assert_eq!(parse_handshake(addr.as_slice()).unwrap(), 0);

        let addr = NetworkAddress::from_str(&format!(
            "/ip4/127.0.0.1/tcp/999/noise-ik/{}/handshake/0/",
            pubkey_str
        ))
        .unwrap();
        assert_eq!(parse_handshake(addr.as_slice()).unwrap(), 0);
    }

    proptest! {
        #[test]
        fn test_network_address_canonical_serialization(addr in arb_aptosnet_addr()) {
            if addr.to_string().is_ascii() {
                assert_canonical_encode_decode(addr);
            } else {
                let addr_bytes = bcs::to_bytes(&addr).unwrap();
                bcs::from_bytes::<NetworkAddress>(&addr_bytes).unwrap_err();
            }
        }

        #[test]
        fn test_network_address_display_roundtrip(addr in arb_aptosnet_addr()) {
            let addr_str = addr.to_string();
            let addr_parsed = NetworkAddress::from_str(&addr_str);
            if addr_str.is_ascii() {
                assert_eq!(addr, addr_parsed.unwrap());
            } else {
                assert_matches!(
                    addr_parsed,
                    Err(ParseError::DnsNameNonASCII(_))
                );
            }
        }

        #[test]
        fn test_is_aptosnet_addr(addr in arb_aptosnet_addr()) {
            assert!(addr.is_aptosnet_addr(), "addr.is_aptosnet_addr() = false; addr: '{}'", addr);
        }
    }
}
