//! Appraisal of the device-evidence shape emitted by the measured Tinfoil guest.
//!
//! The CPU quote binds the *bytes* of this section to the measured workload.
//! The Tinfoil guest appraises its GPUs with `nvattest` at boot before marking
//! them ready. This module checks that the reported device count, architecture,
//! challenge nonce, and (for HGX Hopper) GPU/NVSwitch fabric are consistent
//! with that guest's output. It does **not** verify NVIDIA certificate chains,
//! SPDM signatures, measurements, RIMs, or revocation independently. Call it
//! only after authenticating the CPU quote and the measured guest release.

use crate::envelope::Document;
use base64::Engine;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const GPU_FORMAT: &str = "https://tinfoil.sh/format/nvidia-gpu-evidence/v1";
const NVSWITCH_FORMAT: &str = "https://tinfoil.sh/format/nvidia-nvswitch-evidence/v1";
const HOPPER_GPUS: u32 = 8;
const HOPPER_SWITCHES: u32 = 4;
const SPDM_REQUEST_LEN: usize = 37;
const SPDM_RESPONSE_HEADER_LEN: usize = 8;
const SPDM_NONCE_LEN: usize = 32;

/// Architecture asserted by the authenticated measured guest's evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceArchitecture {
    None,
    Hopper,
    Blackwell,
}

/// GPU shape appraised by the measured guest and structurally checked here.
/// This is **not** an independently NVIDIA-verified device claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceTopology {
    pub gpu_count: u32,
    pub nvswitch_count: u32,
    pub arch: DeviceArchitecture,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum DeviceError {
    #[error("GPU topology is malformed: {0}")]
    Malformed(String),
    #[error("expected {expected} GPU evidence items, got {actual}")]
    GpuCount { expected: u32, actual: u32 },
}

#[derive(Deserialize)]
struct GpuEvidence {
    arch: String,
    certificate: String,
    evidence: String,
    nonce: String,
}

#[derive(Deserialize)]
struct SwitchCollection {
    result_code: i64,
    evidences: Vec<SwitchEvidence>,
}

#[derive(Deserialize)]
struct SwitchEvidence {
    certificate: String,
    evidence: String,
    // nvattest's NVSwitch JSON is not guaranteed to echo --nonce. If it does,
    // check it; otherwise the nonce is bound by the measured guest's request.
    nonce: Option<String>,
}

/// Validate the v3 device section against the signed code shape's GPU count.
///
/// This checks guest-reported evidence structure. It relies on the measured
/// guest's own boot-time NVIDIA appraisal, not a new NVIDIA trust root.
pub fn validate_topology(
    document: &Document,
    expected_gpus: u32,
) -> Result<DeviceTopology, DeviceError> {
    let mut gpus = BTreeMap::new();
    let mut switches = Vec::new();
    for item in &document.device_evidence.items {
        match item.kind.as_str() {
            "gpu" => {
                if item.vendor != "nvidia" || item.format != GPU_FORMAT {
                    return malformed(format!("{} has unsupported GPU vendor or format", item.id));
                }
                let index = item
                    .id
                    .strip_prefix("gpu")
                    .and_then(|s| s.parse::<u32>().ok())
                    .ok_or_else(|| DeviceError::Malformed(format!("invalid GPU id {}", item.id)))?;
                if item.id != format!("gpu{index}") || gpus.insert(index, &item.evidence).is_some()
                {
                    return malformed(format!("duplicate or noncanonical GPU id {}", item.id));
                }
            },
            "nvswitch" => {
                if item.id != "nvswitch0"
                    || item.vendor != "nvidia"
                    || item.format != NVSWITCH_FORMAT
                    || !switches.is_empty()
                {
                    return malformed(format!("unsupported NVSwitch item {}", item.id));
                }
                switches.push(&item.evidence);
            },
            _ => return malformed(format!("unsupported device kind {}", item.kind)),
        }
    }
    let actual =
        u32::try_from(gpus.len()).map_err(|_| DeviceError::Malformed("too many GPUs".into()))?;
    if actual != expected_gpus {
        return Err(DeviceError::GpuCount {
            expected: expected_gpus,
            actual,
        });
    }
    if expected_gpus == 0 {
        if !switches.is_empty() {
            return malformed("CPU-only shape has NVSwitch evidence");
        }
        return Ok(DeviceTopology {
            gpu_count: 0,
            nvswitch_count: 0,
            arch: DeviceArchitecture::None,
        });
    }
    let mut arch = None;
    let mut gpu_reports = Vec::with_capacity(gpus.len());
    for index in 0..expected_gpus {
        let value = gpus
            .get(&index)
            .ok_or_else(|| DeviceError::Malformed(format!("missing gpu{index}")))?;
        let gpu: GpuEvidence = serde_json::from_value((*value).clone())
            .map_err(|err| DeviceError::Malformed(format!("gpu{index}: {err}")))?;
        let this_arch = match gpu.arch.as_str() {
            "HOPPER" => DeviceArchitecture::Hopper,
            "BLACKWELL" => DeviceArchitecture::Blackwell,
            _ => {
                return malformed(format!(
                    "gpu{index} has unsupported architecture {}",
                    gpu.arch
                ))
            },
        };
        if let Some(previous) = arch
            && previous != this_arch
        {
            return malformed("mixed GPU architectures");
        }
        arch = Some(this_arch);
        check_nonce(
            &gpu.nonce,
            &document.raw.challenge.nonce,
            &format!("gpu{index}"),
        )?;
        decode_nonempty(&gpu.certificate, &format!("gpu{index} certificate"))?;
        gpu_reports.push(decode_nonempty(
            &gpu.evidence,
            &format!("gpu{index} report"),
        )?);
    }
    let arch = arch.ok_or_else(|| DeviceError::Malformed("missing GPU architecture".into()))?;
    let required_switches = match arch {
        DeviceArchitecture::Hopper if expected_gpus == 1 => 0,
        DeviceArchitecture::Hopper if expected_gpus == HOPPER_GPUS => HOPPER_SWITCHES,
        DeviceArchitecture::Hopper => {
            return malformed(format!(
                "unsupported Hopper shape with {expected_gpus} GPUs"
            ))
        },
        DeviceArchitecture::Blackwell => 0,
        DeviceArchitecture::None => unreachable!(),
    };
    if required_switches == 0 {
        if !switches.is_empty() {
            return malformed("shape must not have NVSwitch evidence");
        }
    } else {
        let value = switches.first().ok_or_else(|| {
            DeviceError::Malformed("Hopper HGX shape is missing NVSwitch evidence".into())
        })?;
        let collection: SwitchCollection = serde_json::from_value((*value).clone())
            .map_err(|err| DeviceError::Malformed(format!("NVSwitch collection: {err}")))?;
        if collection.result_code != 0 {
            return malformed(format!(
                "NVSwitch collection failed with code {}",
                collection.result_code
            ));
        }
        if collection.evidences.len() != required_switches as usize {
            return malformed(format!(
                "expected {required_switches} NVSwitch reports, got {}",
                collection.evidences.len()
            ));
        }
        let mut switch_reports = Vec::with_capacity(collection.evidences.len());
        for (i, switch) in collection.evidences.iter().enumerate() {
            if let Some(nonce) = &switch.nonce {
                check_nonce(
                    nonce,
                    &document.raw.challenge.nonce,
                    &format!("nvswitch{i}"),
                )?;
            }
            decode_nonempty(&switch.certificate, &format!("nvswitch{i} certificate"))?;
            switch_reports.push(decode_nonempty(
                &switch.evidence,
                &format!("nvswitch{i} report"),
            )?);
        }
        check_hopper_mesh(&gpu_reports, &switch_reports)?;
    }
    Ok(DeviceTopology {
        gpu_count: expected_gpus,
        nvswitch_count: required_switches,
        arch,
    })
}

fn malformed<T>(message: impl Into<String>) -> Result<T, DeviceError> {
    Err(DeviceError::Malformed(message.into()))
}

fn check_nonce(actual: &str, expected: &str, label: &str) -> Result<(), DeviceError> {
    if actual != expected {
        return malformed(format!("{label} nonce does not match the challenge"));
    }
    Ok(())
}

fn decode_nonempty(encoded: &str, label: &str) -> Result<Vec<u8>, DeviceError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| DeviceError::Malformed(format!("{label} is not base64")))?;
    if bytes.is_empty() || base64::engine::general_purpose::STANDARD.encode(&bytes) != encoded {
        return malformed(format!("{label} is empty or noncanonical base64"));
    }
    Ok(bytes)
}

fn opaque_fields<'a>(
    report: &'a [u8],
    label: &str,
) -> Result<BTreeMap<u16, &'a [u8]>, DeviceError> {
    let response = report
        .get(SPDM_REQUEST_LEN..)
        .ok_or_else(|| DeviceError::Malformed(format!("{label}: short SPDM request")))?;
    if response.len() < SPDM_RESPONSE_HEADER_LEN || !matches!(response[0], 0x11 | 0x12) {
        return malformed(format!("{label}: unsupported or truncated SPDM response"));
    }
    let measurement_len = usize::from(response[5])
        | (usize::from(response[6]) << 8)
        | (usize::from(response[7]) << 16);
    let opaque_len_offset = SPDM_RESPONSE_HEADER_LEN + measurement_len + SPDM_NONCE_LEN;
    let size = response
        .get(opaque_len_offset..opaque_len_offset + 2)
        .ok_or_else(|| DeviceError::Malformed(format!("{label}: missing opaque length")))?;
    let opaque_len = usize::from(u16::from_le_bytes([size[0], size[1]]));
    let data = response
        .get(opaque_len_offset + 2..opaque_len_offset + 2 + opaque_len)
        .ok_or_else(|| DeviceError::Malformed(format!("{label}: truncated opaque data")))?;
    let mut fields = BTreeMap::new();
    let mut cursor = 0;
    while cursor < data.len() {
        let header = data.get(cursor..cursor + 4).ok_or_else(|| {
            DeviceError::Malformed(format!("{label}: truncated opaque field header"))
        })?;
        let kind = u16::from_le_bytes([header[0], header[1]]);
        let size = usize::from(u16::from_le_bytes([header[2], header[3]]));
        cursor += 4;
        let value = data.get(cursor..cursor + size).ok_or_else(|| {
            DeviceError::Malformed(format!("{label}: truncated opaque field {kind}"))
        })?;
        if fields.insert(kind, value).is_some() {
            return malformed(format!("{label}: duplicate opaque field {kind}"));
        }
        cursor += size;
    }
    Ok(fields)
}

type Pdi = [u8; 8];

fn pdi_set(
    raw: &[u8],
    count: usize,
    reverse: bool,
    label: &str,
) -> Result<BTreeSet<Pdi>, DeviceError> {
    if raw.len() < count * 8 {
        return malformed(format!("{label}: PDI list is too short"));
    }
    let mut set = BTreeSet::new();
    for mut pdi in raw[..count * 8].as_chunks::<8>().0.iter().copied() {
        if reverse {
            pdi.reverse();
        }
        if pdi != [0; 8] {
            set.insert(pdi);
        }
    }
    Ok(set)
}

fn check_hopper_mesh(
    gpu_reports: &[Vec<u8>],
    switch_reports: &[Vec<u8>],
) -> Result<(), DeviceError> {
    let mut common_switches = None;
    for (i, report) in gpu_reports.iter().enumerate() {
        let label = format!("gpu{i}");
        let fields = opaque_fields(report, &label)?;
        let raw = fields
            .get(&22)
            .ok_or_else(|| DeviceError::Malformed(format!("{label}: missing switch PDI field")))?;
        if raw.len() % 8 != 0 {
            return malformed(format!("{label}: malformed switch PDI field"));
        }
        let set = pdi_set(raw, raw.len() / 8, true, &label)?;
        if set.len() != HOPPER_SWITCHES as usize {
            return malformed(format!(
                "{label}: expected {HOPPER_SWITCHES} distinct switches"
            ));
        }
        if let Some(previous) = &common_switches {
            if previous != &set {
                return malformed(format!("{label}: switch mesh differs from gpu0"));
            }
        } else {
            common_switches = Some(set);
        }
    }
    let common_switches =
        common_switches.ok_or_else(|| DeviceError::Malformed("missing GPU mesh".into()))?;
    let mut seen_switches = BTreeSet::new();
    let mut common_gpus = None;
    for (i, report) in switch_reports.iter().enumerate() {
        let label = format!("nvswitch{i}");
        let fields = opaque_fields(report, &label)?;
        let own = fields
            .get(&22)
            .ok_or_else(|| DeviceError::Malformed(format!("{label}: missing own PDI")))?;
        if own.len() != 8 {
            return malformed(format!("{label}: invalid own PDI"));
        }
        let own: Pdi = (*own)
            .try_into()
            .map_err(|_| DeviceError::Malformed(format!("{label}: malformed own PDI")))?;
        if !common_switches.contains(&own) || !seen_switches.insert(own) {
            return malformed(format!(
                "{label}: own PDI is absent from or duplicated in GPU mesh"
            ));
        }
        let raw = fields
            .get(&26)
            .ok_or_else(|| DeviceError::Malformed(format!("{label}: missing GPU PDI field")))?;
        let set = pdi_set(raw, HOPPER_GPUS as usize, false, &label)?;
        if set.len() != HOPPER_GPUS as usize {
            return malformed(format!("{label}: expected {HOPPER_GPUS} distinct GPUs"));
        }
        if let Some(previous) = &common_gpus {
            if previous != &set {
                return malformed(format!("{label}: GPU mesh differs from nvswitch0"));
            }
        } else {
            common_gpus = Some(set);
        }
    }
    if seen_switches != common_switches {
        return malformed("NVSwitch reports do not cover the full GPU-reported mesh");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata::DocumentBuilder;
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};

    fn fixture(items: Vec<Value>) -> Document {
        let mut value: Value = serde_json::from_str(&DocumentBuilder::new().build().json).unwrap();
        value["device_evidence"] = Value::String(
            base64::engine::general_purpose::STANDARD.encode(
                serde_json::to_vec(
                    &json!({"format": crate::envelope::DEVICE_EVIDENCE_V1_FORMAT, "items": items}),
                )
                .unwrap(),
            ),
        );
        Document::parse(serde_json::to_string(&value).unwrap().as_bytes()).unwrap()
    }

    fn gpu(index: u32, arch: &str, report: &[u8]) -> Value {
        json!({"id": format!("gpu{index}"), "kind":"gpu", "vendor":"nvidia", "format":GPU_FORMAT,
            "evidence":{"arch":arch,"certificate":base64::engine::general_purpose::STANDARD.encode([1,2,3]),
                        "evidence":base64::engine::general_purpose::STANDARD.encode(report),"nonce":"42".repeat(32)}})
    }

    fn spdm(fields: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut opaque = Vec::new();
        for (kind, value) in fields {
            opaque.extend(kind.to_le_bytes());
            opaque.extend((value.len() as u16).to_le_bytes());
            opaque.extend(value);
        }
        let mut result = vec![0; SPDM_REQUEST_LEN + SPDM_RESPONSE_HEADER_LEN + SPDM_NONCE_LEN];
        result[SPDM_REQUEST_LEN] = 0x12;
        result.extend((opaque.len() as u16).to_le_bytes());
        result.extend(opaque);
        result
    }

    #[test]
    fn single_hopper_and_blackwell_shapes() {
        let hopper = fixture(vec![gpu(0, "HOPPER", &[1])]);
        assert_eq!(validate_topology(&hopper, 1).unwrap(), DeviceTopology {
            gpu_count: 1,
            nvswitch_count: 0,
            arch: DeviceArchitecture::Hopper
        });
        let blackwell = fixture(vec![gpu(0, "BLACKWELL", &[1]), gpu(1, "BLACKWELL", &[2])]);
        assert_eq!(
            validate_topology(&blackwell, 2).unwrap().arch,
            DeviceArchitecture::Blackwell
        );
        assert!(validate_topology(&blackwell, 1).is_err());
        assert!(validate_topology(&fixture(vec![]), 0).is_ok());
    }

    #[test]
    fn hopper_hgx_mesh() {
        let switches: Vec<Pdi> = (1u8..=4)
            .map(|i| std::array::from_fn(|offset| i + offset as u8))
            .collect();
        let gpus: Vec<Pdi> = (11u8..=18)
            .map(|i| std::array::from_fn(|offset| i + offset as u8))
            .collect();
        let mut switch_list = Vec::new();
        for pdi in &switches {
            let mut reversed = *pdi;
            reversed.reverse();
            switch_list.extend(reversed);
        }
        let gpu_report = spdm(&[(22, switch_list)]);
        let mut items: Vec<Value> = (0..8).map(|i| gpu(i, "HOPPER", &gpu_report)).collect();
        let reports: Vec<Value> = switches.iter().map(|switch| {
            let mut gpu_list = Vec::new();
            for gpu in &gpus { gpu_list.extend(gpu); }
            // The Tinfoil boot parser only requires these two per-device fields;
            // nvattest versions may omit a separate nonce JSON member.
            json!({"certificate":base64::engine::general_purpose::STANDARD.encode([1]),
                   "evidence":base64::engine::general_purpose::STANDARD.encode(spdm(&[(22, switch.to_vec()), (26, gpu_list)]))})
        }).collect();
        items.push(
            json!({"id":"nvswitch0","kind":"nvswitch","vendor":"nvidia","format":NVSWITCH_FORMAT,
            "evidence":{"result_code":0,"evidences":reports}}),
        );
        let document = fixture(items.clone());
        assert_eq!(validate_topology(&document, 8).unwrap().nvswitch_count, 4);
        let mut bad_mesh = items.clone();
        bad_mesh.last_mut().unwrap()["evidence"]["evidences"][0]["evidence"] =
            json!(base64::engine::general_purpose::STANDARD.encode(spdm(&[
                (22, [99; 8].to_vec()),
                (26, gpus.iter().flatten().copied().collect())
            ])));
        assert!(validate_topology(&fixture(bad_mesh), 8).is_err());
        items.last_mut().unwrap()["evidence"]["result_code"] = json!(2);
        assert!(validate_topology(&fixture(items), 8).is_err());
    }

    #[test]
    fn rejects_nonce_and_architecture_mismatches() {
        let mut item = gpu(0, "HOPPER", &[1]);
        item["evidence"]["nonce"] = json!("00".repeat(32));
        assert!(validate_topology(&fixture(vec![item]), 1).is_err());
        assert!(validate_topology(
            &fixture(vec![gpu(0, "HOPPER", &[1]), gpu(1, "BLACKWELL", &[2])]),
            2
        )
        .is_err());
        assert!(validate_topology(
            &fixture(vec![gpu(0, "HOPPER", &[1]), gpu(2, "HOPPER", &[2])]),
            2
        )
        .is_err());
    }

    #[test]
    fn gpu_evidence_is_bound_to_the_cpu_stage_commitment() {
        let first = DocumentBuilder::new()
            .device_item(gpu(0, "HOPPER", &[1]))
            .build();
        let (document, report_data) = Document::check(first.json.as_bytes(), &first.nonce).unwrap();
        assert_eq!(validate_topology(&document, 1).unwrap().gpu_count, 1);
        let quote_sha256: [u8; 32] =
            Sha256::digest(document.raw.cpu_evidence.report_base64.as_bytes()).into();

        let second = DocumentBuilder::new()
            .device_item(gpu(0, "HOPPER", &[2]))
            .build();
        assert!(matches!(
            crate::check_fragment(
                second.json.as_bytes(),
                &first.nonce,
                Some((&report_data, &quote_sha256)),
            ),
            Err(crate::VerifyError::StageBindingMismatch)
        ));
    }
}
