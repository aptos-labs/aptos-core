use crate::{
    idl::JwkIdlError,
    jwks::{jwk::JWKMoveStruct, AllProvidersJWKs, ObservedJWKs, ProviderJWKs},
    move_any::Any,
    on_chain_config::OIDCProvider,
};
use anyhow::{anyhow, format_err};
pub fn convert_provider_jwks(
    origin_provider_jwks: api_types::on_chain_config::jwks::ProviderJWKs,
) -> ProviderJWKs {
    let api_types::on_chain_config::jwks::ProviderJWKs {
        issuer,
        version,
        jwks,
    } = origin_provider_jwks;
    ProviderJWKs {
        issuer,
        version,
        jwks: jwks
            .iter()
            .map(|jwk| JWKMoveStruct {
                variant: Any {
                    type_name: jwk.type_name.clone(),
                    data: jwk.data.clone(),
                },
            })
            .collect(),
    }
}

pub fn convert_all_providers_jwks(
    origin_all_providers_jwks: api_types::on_chain_config::jwks::AllProvidersJWKs,
) -> AllProvidersJWKs {
    let api_types::on_chain_config::jwks::AllProvidersJWKs { entries } = origin_all_providers_jwks;
    AllProvidersJWKs {
        entries: entries
            .iter()
            .map(|origin_provider_jwks| convert_provider_jwks(origin_provider_jwks.clone()))
            .collect(),
    }
}

pub fn convert_observed_jwks(
    origin_all_providers_jwks: api_types::on_chain_config::jwks::ObservedJWKs,
) -> ObservedJWKs {
    ObservedJWKs {
        jwks: convert_all_providers_jwks(origin_all_providers_jwks.jwks),
    }
}

pub fn construct_observed_jwks(bytes: &[u8]) -> Result<ObservedJWKs, JwkIdlError> {
    let observed_jwks = bcs::from_bytes::<api_types::on_chain_config::jwks::ObservedJWKs>(bytes)
        .map_err(|e| JwkIdlError::JsonDeserializationError(e.to_string()))?;
    Ok(convert_observed_jwks(observed_jwks))
}

pub fn construct_oidc_providers(bytes: &[u8]) -> Result<Vec<OIDCProvider>, JwkIdlError> {
    let oidc_providers =
        bcs::from_bytes::<Vec<api_types::on_chain_config::jwks::OIDCProvider>>(bytes)
            .map_err(|e| JwkIdlError::JsonDeserializationError(e.to_string()))?;
    Ok(oidc_providers
        .iter()
        .map(|oidc_provider| OIDCProvider {
            name: oidc_provider.name.clone(),
            config_url: oidc_provider.config_url.clone(),
        })
        .collect())
}
