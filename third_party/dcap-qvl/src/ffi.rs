use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::slice;

use scale::Decode;
use serde::Serialize;

use crate::intel;
use crate::quote::{EnclaveReport, Header, Quote, Report, TDReport10, TDReport15};
use crate::verify::{self, VerifiedReport};
use crate::QuoteCollateralV3;

// ---------------------------------------------------------------------------
// FFI-specific serialization structs (flattened for Go consumption)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct FfiQuote {
    header: Header,
    report: FfiReport,
    cert_type: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    cert_chain_pem: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fmspc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ca: Option<String>,
    quote_type: &'static str,
    // cert_type 2/3: encrypted PPID params for PCK cert fetch
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    qe_id: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    encrypted_ppid: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    cert_cpusvn: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cert_pcesvn: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    cert_pceid: Option<Vec<u8>>,
}

#[derive(Serialize)]
struct FfiReport {
    r#type: &'static str,
    #[serde(with = "serde_bytes")]
    report_data: Vec<u8>,
    // TDX fields
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    tee_tcb_svn: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_seam: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_signer_seam: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    seam_attributes: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    td_attributes: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    xfam: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_td: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_config_id: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_owner: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_owner_config: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    rt_mr0: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    rt_mr1: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    rt_mr2: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    rt_mr3: Option<Vec<u8>>,
    // TD15 extra
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    tee_tcb_svn2: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_service_td: Option<Vec<u8>>,
    // SGX fields
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    cpu_svn: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    misc_select: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    attributes: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_enclave: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::ffi::opt_hex")]
    mr_signer: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    isv_prod_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    isv_svn: Option<u16>,
}

/// serde helper for Option<Vec<u8>> serialized as hex via serde_bytes when Some
mod opt_hex {
    use serde::Serializer;

    pub fn serialize<S: Serializer>(
        value: &Option<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => serde_bytes::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }
}

impl FfiReport {
    fn from_td10(r: &TDReport10) -> Self {
        Self {
            r#type: "TD10",
            report_data: r.report_data.to_vec(),
            tee_tcb_svn: Some(r.tee_tcb_svn.to_vec()),
            mr_seam: Some(r.mr_seam.to_vec()),
            mr_signer_seam: Some(r.mr_signer_seam.to_vec()),
            seam_attributes: Some(r.seam_attributes.to_vec()),
            td_attributes: Some(r.td_attributes.to_vec()),
            xfam: Some(r.xfam.to_vec()),
            mr_td: Some(r.mr_td.to_vec()),
            mr_config_id: Some(r.mr_config_id.to_vec()),
            mr_owner: Some(r.mr_owner.to_vec()),
            mr_owner_config: Some(r.mr_owner_config.to_vec()),
            rt_mr0: Some(r.rt_mr0.to_vec()),
            rt_mr1: Some(r.rt_mr1.to_vec()),
            rt_mr2: Some(r.rt_mr2.to_vec()),
            rt_mr3: Some(r.rt_mr3.to_vec()),
            tee_tcb_svn2: None,
            mr_service_td: None,
            cpu_svn: None,
            misc_select: None,
            attributes: None,
            mr_enclave: None,
            mr_signer: None,
            isv_prod_id: None,
            isv_svn: None,
        }
    }

    fn from_td15(r: &TDReport15) -> Self {
        let mut ffi = Self::from_td10(&r.base);
        ffi.r#type = "TD15";
        ffi.tee_tcb_svn2 = Some(r.tee_tcb_svn2.to_vec());
        ffi.mr_service_td = Some(r.mr_service_td.to_vec());
        ffi
    }

    fn from_sgx(r: &EnclaveReport) -> Self {
        Self {
            r#type: "SGX",
            report_data: r.report_data.to_vec(),
            cpu_svn: Some(r.cpu_svn.to_vec()),
            misc_select: Some(r.misc_select),
            attributes: Some(r.attributes.to_vec()),
            mr_enclave: Some(r.mr_enclave.to_vec()),
            mr_signer: Some(r.mr_signer.to_vec()),
            isv_prod_id: Some(r.isv_prod_id),
            isv_svn: Some(r.isv_svn),
            tee_tcb_svn: None,
            mr_seam: None,
            mr_signer_seam: None,
            seam_attributes: None,
            td_attributes: None,
            xfam: None,
            mr_td: None,
            mr_config_id: None,
            mr_owner: None,
            mr_owner_config: None,
            rt_mr0: None,
            rt_mr1: None,
            rt_mr2: None,
            rt_mr3: None,
            tee_tcb_svn2: None,
            mr_service_td: None,
        }
    }

    fn from_report(report: &Report) -> Self {
        match report {
            Report::SgxEnclave(r) => Self::from_sgx(r),
            Report::TD10(r) => Self::from_td10(r),
            Report::TD15(r) => Self::from_td15(r),
        }
    }
}

#[derive(Serialize)]
struct FfiVerifiedReport {
    status: String,
    advisory_ids: Vec<String>,
    report: FfiReport,
    #[serde(with = "serde_bytes")]
    ppid: Vec<u8>,
    qe_status: crate::tcb_info::TcbStatusWithAdvisory,
    platform_status: crate::tcb_info::TcbStatusWithAdvisory,
}

impl FfiVerifiedReport {
    fn from(vr: VerifiedReport) -> Self {
        Self {
            status: vr.status,
            advisory_ids: vr.advisory_ids,
            report: FfiReport::from_report(&vr.report),
            ppid: vr.ppid,
            qe_status: vr.qe_status,
            platform_status: vr.platform_status,
        }
    }
}

#[derive(Serialize)]
struct FfiPckExtension {
    #[serde(with = "serde_bytes")]
    ppid: Vec<u8>,
    #[serde(with = "serde_bytes")]
    cpu_svn: Vec<u8>,
    pce_svn: u16,
    #[serde(with = "serde_bytes")]
    pce_id: Vec<u8>,
    #[serde(with = "serde_bytes")]
    fmspc: Vec<u8>,
    sgx_type: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    platform_instance_id: Option<serde_bytes::ByteBuf>,
    #[serde(with = "serde_bytes")]
    raw_extension: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type OutputCallback = Option<unsafe extern "C" fn(*const u8, usize, *mut c_void) -> i32>;

const ERR_GENERIC: i32 = 1;
const ERR_CALLBACK: i32 = 2;

unsafe fn emit_output(json: String, cb: OutputCallback, user_data: *mut c_void) -> i32 {
    let Some(callback) = cb else {
        return ERR_CALLBACK;
    };

    let bytes = json.as_bytes();
    let rc = callback(bytes.as_ptr(), bytes.len(), user_data);
    if rc == 0 {
        0
    } else {
        ERR_CALLBACK
    }
}

unsafe fn emit_error(msg: String, cb: OutputCallback, user_data: *mut c_void) -> i32 {
    let rc = emit_output(msg, cb, user_data);
    if rc == 0 {
        ERR_GENERIC
    } else {
        rc
    }
}

fn format_error(e: &anyhow::Error) -> String {
    let mut msg = e.to_string();
    let mut source = e.source();
    while let Some(err) = source {
        msg.push_str(": ");
        msg.push_str(&err.to_string());
        source = err.source();
    }
    msg
}

// ---------------------------------------------------------------------------
// extern "C" functions
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn dcap_parse_quote_cb(
    quote: *const u8,
    quote_len: usize,
    cb: OutputCallback,
    user_data: *mut c_void,
) -> i32 {
    let quote_slice = slice::from_raw_parts(quote, quote_len);
    let parsed = match Quote::decode(&mut &quote_slice[..]) {
        Ok(q) => q,
        Err(e) => return emit_error(format!("Failed to parse quote: {e}"), cb, user_data),
    };

    let cert_type = parsed.inner_cert_type();
    let quote_type = if parsed.header.is_sgx() { "SGX" } else { "TDX" };

    let cert_chain_pem = parsed.raw_cert_chain().ok().map(|raw| {
        let trimmed = raw
            .iter()
            .rposition(|byte| *byte != 0)
            .and_then(|end| raw.get(..=end))
            .unwrap_or_default();
        String::from_utf8_lossy(trimmed).into_owned()
    });

    // For cert_type 5: extract fmspc/ca from embedded cert chain
    // For cert_type 2/3: these are unavailable until PCK cert is fetched
    let (fmspc, ca) = if cert_chain_pem.is_some() {
        let fmspc = match crate::intel::quote_fmspc(&parsed) {
            Ok(f) => Some(hex::encode_upper(f)),
            Err(e) => return emit_error(format_error(&e), cb, user_data),
        };
        let ca = match crate::intel::quote_ca(&parsed) {
            Ok(c) => Some(c.as_id_str().to_string()),
            Err(e) => return emit_error(format_error(&e), cb, user_data),
        };
        (fmspc, ca)
    } else {
        (None, None)
    };

    // For cert_type 2/3: extract encrypted PPID params
    let (qe_id, encrypted_ppid, cert_cpusvn, cert_pcesvn, cert_pceid) =
        if let Ok(params) = parsed.encrypted_ppid_params() {
            (
                Some(parsed.qeid().to_vec()),
                Some(params.encrypted_ppid),
                Some(params.cpusvn.to_vec()),
                Some(params.pcesvn),
                Some(params.pceid.to_vec()),
            )
        } else {
            (None, None, None, None, None)
        };

    let ffi_quote = FfiQuote {
        header: parsed.header,
        report: FfiReport::from_report(&parsed.report),
        cert_type,
        cert_chain_pem,
        fmspc,
        ca,
        quote_type,
        qe_id,
        encrypted_ppid,
        cert_cpusvn,
        cert_pcesvn,
        cert_pceid,
    };

    let json = match serde_json::to_string(&ffi_quote) {
        Ok(j) => j,
        Err(e) => return emit_error(format!("JSON serialization failed: {e}"), cb, user_data),
    };

    emit_output(json, cb, user_data)
}

#[no_mangle]
pub unsafe extern "C" fn dcap_verify_cb(
    quote: *const u8,
    quote_len: usize,
    collateral_json: *const u8,
    coll_len: usize,
    now_secs: u64,
    cb: OutputCallback,
    user_data: *mut c_void,
) -> i32 {
    let quote_slice = slice::from_raw_parts(quote, quote_len);
    let coll_slice = slice::from_raw_parts(collateral_json, coll_len);

    let collateral: QuoteCollateralV3 = match serde_json::from_slice(coll_slice) {
        Ok(c) => c,
        Err(e) => {
            return emit_error(
                format!("Failed to parse collateral JSON: {e}"),
                cb,
                user_data,
            )
        }
    };

    let report = match verify::verify(quote_slice, &collateral, now_secs) {
        Ok(r) => r,
        Err(e) => return emit_error(format_error(&e), cb, user_data),
    };

    let ffi_report = FfiVerifiedReport::from(report);
    let json = match serde_json::to_string(&ffi_report) {
        Ok(j) => j,
        Err(e) => return emit_error(format!("JSON serialization failed: {e}"), cb, user_data),
    };

    emit_output(json, cb, user_data)
}

#[no_mangle]
pub unsafe extern "C" fn dcap_verify_with_root_ca_cb(
    quote: *const u8,
    quote_len: usize,
    collateral_json: *const u8,
    coll_len: usize,
    root_ca_der: *const u8,
    root_ca_len: usize,
    now_secs: u64,
    cb: OutputCallback,
    user_data: *mut c_void,
) -> i32 {
    let quote_slice = slice::from_raw_parts(quote, quote_len);
    let coll_slice = slice::from_raw_parts(collateral_json, coll_len);
    let root_ca = slice::from_raw_parts(root_ca_der, root_ca_len);

    let collateral: QuoteCollateralV3 = match serde_json::from_slice(coll_slice) {
        Ok(c) => c,
        Err(e) => {
            return emit_error(
                format!("Failed to parse collateral JSON: {e}"),
                cb,
                user_data,
            )
        }
    };

    let verifier = verify::QuoteVerifier::new(root_ca.to_vec());

    let report = match verifier.verify(quote_slice, &collateral, now_secs) {
        Ok(r) => r,
        Err(e) => return emit_error(format_error(&e), cb, user_data),
    };

    let ffi_report = FfiVerifiedReport::from(report);
    let json = match serde_json::to_string(&ffi_report) {
        Ok(j) => j,
        Err(e) => return emit_error(format!("JSON serialization failed: {e}"), cb, user_data),
    };

    emit_output(json, cb, user_data)
}

#[cfg(all(feature = "report", feature = "tokio"))]
#[no_mangle]
pub unsafe extern "C" fn dcap_get_collateral_cb(
    pccs_url: *const u8,
    url_len: usize,
    quote: *const u8,
    quote_len: usize,
    cb: OutputCallback,
    user_data: *mut c_void,
) -> i32 {
    let url_slice = slice::from_raw_parts(pccs_url, url_len);
    let url = match core::str::from_utf8(url_slice) {
        Ok(s) => s,
        Err(e) => return emit_error(format!("Invalid URL: {e}"), cb, user_data),
    };
    let quote_slice = slice::from_raw_parts(quote, quote_len);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return emit_error(format!("Failed to create runtime: {e}"), cb, user_data),
    };

    let collateral = match rt.block_on(async {
        crate::collateral::CollateralClient::with_default_http(url)?
            .fetch(quote_slice)
            .await
    }) {
        Ok(c) => c,
        Err(e) => return emit_error(format_error(&e), cb, user_data),
    };

    let json = match serde_json::to_string(&collateral) {
        Ok(j) => j,
        Err(e) => return emit_error(format!("JSON serialization failed: {e}"), cb, user_data),
    };

    emit_output(json, cb, user_data)
}

#[no_mangle]
pub unsafe extern "C" fn dcap_parse_pck_extension_from_pem_cb(
    pem: *const u8,
    pem_len: usize,
    cb: OutputCallback,
    user_data: *mut c_void,
) -> i32 {
    let pem_slice = slice::from_raw_parts(pem, pem_len);

    let ext = match intel::parse_pck_extension_from_pem(pem_slice) {
        Ok(e) => e,
        Err(e) => return emit_error(format!("Failed to parse PCK extension: {e}"), cb, user_data),
    };

    let ffi_ext = FfiPckExtension {
        ppid: ext.ppid,
        cpu_svn: ext.cpu_svn.to_vec(),
        pce_svn: ext.pce_svn,
        pce_id: ext.pce_id.to_vec(),
        fmspc: ext.fmspc.to_vec(),
        sgx_type: ext.sgx_type,
        platform_instance_id: ext.platform_instance_id.map(serde_bytes::ByteBuf::from),
        raw_extension: ext.raw_extension,
    };

    let json = match serde_json::to_string(&ffi_ext) {
        Ok(j) => j,
        Err(e) => return emit_error(format!("JSON serialization failed: {e}"), cb, user_data),
    };

    emit_output(json, cb, user_data)
}
