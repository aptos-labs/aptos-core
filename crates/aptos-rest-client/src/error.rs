// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

#[derive(Debug)]
pub struct Error {
    inner: Box<Inner>,
}

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug)]
struct Inner {
    kind: Kind,
    source: Option<BoxError>,
}

#[derive(Debug)]
enum Kind {
    HttpStatus(u16),
    Timeout,
    Request,
    RpcResponse,
    ChainId,
    StaleResponse,
    Batch,
    Decode,
    InvalidProof,
    NeedSync,
    StateStore,
    Unknown,
}

impl Error {
    pub fn is_retriable(&self) -> bool {
        match self.inner.kind {
            // internal server errors are retriable
            Kind::HttpStatus(status) => (500..=599).contains(&status),
            Kind::Timeout | Kind::StaleResponse | Kind::NeedSync => true,
            Kind::RpcResponse
            | Kind::Request
            | Kind::ChainId
            | Kind::Batch
            | Kind::Decode
            | Kind::InvalidProof
            | Kind::StateStore
            | Kind::Unknown => false,
        }
    }

    pub fn is_need_sync(&self) -> bool {
        matches!(self.inner.kind, Kind::NeedSync)
    }

    //
    // Private Constructors
    //

    fn new<E: Into<BoxError>>(kind: Kind, source: Option<E>) -> Self {
        Self {
            inner: Box::new(Inner {
                kind,
                source: source.map(Into::into),
            }),
        }
    }

    pub fn status(status: u16) -> Self {
        Self::new(Kind::HttpStatus(status), None::<Error>)
    }

    pub fn timeout<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Timeout, Some(e))
    }

    pub fn rpc_response<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::RpcResponse, Some(e))
    }

    pub fn batch<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Batch, Some(e))
    }

    pub fn decode<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Decode, Some(e))
    }

    pub fn encode<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Decode, Some(e))
    }

    pub fn invalid_proof<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::InvalidProof, Some(e))
    }

    pub fn state_store<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::StateStore, Some(e))
    }

    pub fn need_sync<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::NeedSync, Some(e))
    }

    pub fn unknown<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Unknown, Some(e))
    }

    pub fn request<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Request, Some(e))
    }

    pub fn chain_id(expected: u8, recieved: u8) -> Self {
        Self::new(
            Kind::ChainId,
            Some(format!("expected: {} recieved: {}", expected, recieved)),
        )
    }

    pub fn stale<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::StaleResponse, Some(e))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source.as_ref().map(|e| &**e as _)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::decode(e)
    }
}
