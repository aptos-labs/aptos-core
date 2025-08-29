use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct JWKStruct {
    pub type_name: String,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ProviderJWKs {
    #[serde(with = "serde_bytes")]
    pub issuer: Vec<u8>,
    pub version: u64,
    pub jwks: Vec<JWKStruct>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AllProvidersJWKs {
    pub entries: Vec<ProviderJWKs>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ObservedJWKs {
    pub jwks: AllProvidersJWKs,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OIDCProvider {
    pub name: String,
    pub config_url: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct JWKConsensusConfig {
    pub enabled: bool,
    pub oidc_providers: Vec<OIDCProvider>,
}