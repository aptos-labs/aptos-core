// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bounded observation of governed external trust material. The format number selects
//! code compiled into every validator; target configuration cannot supply code.
//! DER certificate normalization checks the Certificate's top-level ASN.1 shape, but
//! does not validate its signature, chain, expiry, or Intel-specific semantics.

use anyhow::{anyhow, bail, ensure, Context, Result};
use aptos_channels::aptos_channel;
use aptos_logger::{debug, info, warn};
use aptos_types::trust_observation::{TargetSpec, MAX_VALUE_BYTES};
pub use aptos_types::trust_observation::{
    FORMAT_CANONICAL_JSON as CANONICAL_JSON_FORMAT,
    FORMAT_DER_CERTIFICATE as DER_CERTIFICATE_FORMAT, FORMAT_RAW_BYTES as RAW_BYTES_FORMAT,
};
use futures::{FutureExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::{sync::oneshot, task::JoinHandle, time::MissedTickBehavior};
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

pub struct TrustObserver {
    close_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl TrustObserver {
    pub fn spawn(
        epoch: u64,
        target: TargetSpec,
        interval: Duration,
        observation_tx: aptos_channel::Sender<Vec<u8>, (Vec<u8>, Vec<u8>)>,
        extra_root_certificates_pem: Vec<String>,
    ) -> Self {
        let (close_tx, close_rx) = oneshot::channel();
        let target_id = target.id.clone();
        let join_handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let mut close_rx = close_rx.into_stream();
            loop {
                tokio::select! {
                    _ = ticker.tick().fuse() => {
                        match fetch_target(&target, &extra_root_certificates_pem).await {
                            Ok(value) => {
                                let id = target.id.clone();
                                let _ = observation_tx.push(id.clone(), (id, value));
                            },
                            Err(error) => {
                                warn!(target_id = ?target.id, "Trust observation failed: {error:#}");
                            },
                        }
                    },
                    _ = close_rx.select_next_some() => break,
                }
            }
        });
        info!(epoch = epoch, target_id = ?target_id, "Trust observer spawned");
        Self {
            close_tx,
            join_handle,
        }
    }

    pub async fn shutdown(self) {
        let _ = self.close_tx.send(());
        let _ = self.join_handle.await;
    }
}

pub async fn fetch_target(
    target: &TargetSpec,
    extra_root_certificates_pem: &[String],
) -> Result<Vec<u8>> {
    let url = reqwest::Url::parse(&target.url).context("invalid trust target URL")?;
    ensure!(url.scheme() == "https", "trust target must use HTTPS");
    ensure!(url.host_str().is_some(), "trust target URL has no host");
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "credentials in URL forbidden"
    );
    let limit = usize::try_from(target.max_bytes.min(MAX_VALUE_BYTES as u64))?;
    ensure!(limit > 0, "trust target has zero byte limit");
    let mut client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(FETCH_TIMEOUT);
    for pem in extra_root_certificates_pem {
        let certificate = reqwest::Certificate::from_pem(pem.as_bytes())
            .context("invalid extra trust observation root certificate PEM")?;
        client_builder = client_builder.add_root_certificate(certificate);
    }
    let client = client_builder.build()?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    if let Some(len) = response.content_length() {
        ensure!(
            len <= limit as u64,
            "trust target response exceeds byte limit"
        );
    }
    let mut raw = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            chunk.len() <= limit.saturating_sub(raw.len()),
            "trust target response exceeds byte limit"
        );
        raw.extend_from_slice(&chunk);
    }
    let value = canonicalize(target.format, &raw)?;
    ensure!(
        value.len() <= limit,
        "canonical trust value exceeds byte limit"
    );
    debug!(target_id = ?target.id, byte_len = value.len(), "Trust target observed");
    Ok(value)
}

pub fn canonicalize(format: u8, raw: &[u8]) -> Result<Vec<u8>> {
    match format {
        DER_CERTIFICATE_FORMAT => {
            validate_der_certificate_shape(raw)?;
            Ok(raw.to_vec())
        },
        CANONICAL_JSON_FORMAT => {
            let value: Value = serde_json::from_slice(raw).context("invalid JSON trust target")?;
            ensure!(value.is_object(), "JSON trust target must be an object");
            let mut out = Vec::new();
            write_canonical_json(&value, &mut out)?;
            Ok(out)
        },
        RAW_BYTES_FORMAT => {
            ensure!(!raw.is_empty(), "raw trust target must not be empty");
            Ok(raw.to_vec())
        },
        _ => bail!("unknown trust target format {format}"),
    }
}

fn validate_der_certificate_shape(input: &[u8]) -> Result<()> {
    let (certificate_tag, fields, rest) = read_der_tlv(input)?;
    ensure!(
        certificate_tag == 0x30 && rest.is_empty(),
        "DER certificate must be one SEQUENCE"
    );
    let (tbs_tag, tbs, fields) = read_der_tlv(fields)?;
    ensure!(tbs_tag == 0x30 && !tbs.is_empty(), "missing TBSCertificate");
    let (algorithm_tag, algorithm, fields) = read_der_tlv(fields)?;
    ensure!(
        algorithm_tag == 0x30 && !algorithm.is_empty(),
        "missing signatureAlgorithm"
    );
    let (oid_tag, oid, _) = read_der_tlv(algorithm)?;
    ensure!(
        oid_tag == 0x06 && !oid.is_empty(),
        "signatureAlgorithm lacks an OID"
    );
    let (signature_tag, signature, rest) = read_der_tlv(fields)?;
    ensure!(
        signature_tag == 0x03 && rest.is_empty(),
        "missing signatureValue"
    );
    ensure!(signature.len() >= 2, "empty signature BIT STRING");
    let unused_bits = signature[0];
    ensure!(unused_bits <= 7, "invalid signature BIT STRING padding");
    if unused_bits > 0 {
        ensure!(
            signature.last().unwrap() & ((1u8 << unused_bits) - 1) == 0,
            "nonzero signature BIT STRING padding"
        );
    }
    Ok(())
}

fn read_der_tlv(input: &[u8]) -> Result<(u8, &[u8], &[u8])> {
    ensure!(input.len() >= 2, "truncated DER element");
    let tag = input[0];
    ensure!(tag & 0x1F != 0x1F, "high-tag-number DER is unsupported");
    let len_octet = input[1];
    let (header_len, value_len) = if len_octet < 0x80 {
        (2, len_octet as usize)
    } else {
        let width = (len_octet & 0x7F) as usize;
        ensure!(width > 0 && width <= 4, "invalid DER length width");
        ensure!(input.len() >= 2 + width, "truncated DER length");
        ensure!(input[2] != 0, "nonminimal DER length");
        let mut len = 0usize;
        for byte in &input[2..2 + width] {
            len = len
                .checked_mul(256)
                .and_then(|v| v.checked_add(*byte as usize))
                .ok_or_else(|| anyhow!("DER length overflow"))?;
        }
        ensure!(len >= 128, "nonminimal DER length");
        (2 + width, len)
    };
    let end = header_len
        .checked_add(value_len)
        .ok_or_else(|| anyhow!("DER length overflow"))?;
    ensure!(end <= input.len(), "truncated DER element value");
    Ok((tag, &input[header_len..end], &input[end..]))
}

fn write_canonical_json(value: &Value, out: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Object(map) => {
            out.push(b'{');
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key)?;
                out.push(b':');
                write_canonical_json(value, out)?;
            }
            out.push(b'}');
        },
        Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_canonical_json(item, out)?;
            }
            out.push(b']');
        },
        scalar => serde_json::to_writer(out, scalar)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_object_key_order_is_irrelevant() {
        let first = br#"{"z":{"b":1,"a":2},"a":[{"y":true,"x":null}]}"#;
        let second = br#"{ "a": [ { "x": null, "y": true } ], "z": { "a": 2, "b": 1 } }"#;
        assert_eq!(
            canonicalize(CANONICAL_JSON_FORMAT, first).unwrap(),
            canonicalize(CANONICAL_JSON_FORMAT, second).unwrap()
        );
        assert_eq!(
            canonicalize(CANONICAL_JSON_FORMAT, first).unwrap(),
            br#"{"a":[{"x":null,"y":true}],"z":{"a":2,"b":1}}"#
        );
    }

    #[test]
    fn invalid_format_and_der_are_rejected() {
        assert!(canonicalize(99, b"anything").is_err());
        assert_eq!(
            canonicalize(RAW_BYTES_FORMAT, b"\x00\x01").unwrap(),
            b"\x00\x01"
        );
        assert!(canonicalize(RAW_BYTES_FORMAT, b"").is_err());
        assert!(canonicalize(DER_CERTIFICATE_FORMAT, &[0x30, 0x01, 0x00, 0x00]).is_err());
        assert!(canonicalize(DER_CERTIFICATE_FORMAT, &[0x30, 0x80, 0x00, 0x00]).is_err());
        assert!(canonicalize(DER_CERTIFICATE_FORMAT, &[0x30, 0x00]).is_err());
        let certificate = [
            0x30, 0x0E, // Certificate SEQUENCE
            0x30, 0x02, 0x02, 0x00, // nonempty TBSCertificate SEQUENCE
            0x30, 0x03, 0x06, 0x01, 0x2A, // signatureAlgorithm with OID
            0x03, 0x03, 0x00, 0xAA, 0xBB, // signatureValue BIT STRING
        ];
        assert_eq!(
            canonicalize(DER_CERTIFICATE_FORMAT, &certificate).unwrap(),
            certificate
        );
        let mut missing_signature = certificate;
        missing_signature[11] = 0x00;
        assert!(canonicalize(DER_CERTIFICATE_FORMAT, &missing_signature).is_err());
    }

    #[tokio::test]
    async fn observer_rejects_insecure_sources_before_network_access() {
        let target = TargetSpec {
            id: b"root".to_vec(),
            url: "http://127.0.0.1/root.der".into(),
            format: DER_CERTIFICATE_FORMAT,
            max_bytes: 128,
            config_version: 1,
        };
        assert!(fetch_target(&target, &[])
            .await
            .unwrap_err()
            .to_string()
            .contains("HTTPS"));
    }
}
