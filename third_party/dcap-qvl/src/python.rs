use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_async_runtimes::tokio::future_into_py;
use serde_json;
use std::time::Duration;

use crate::{
    collateral::CollateralClient,
    intel,
    policy::{QuoteClaims, QuotePolicy},
    quote::{EnclaveReport, Header, Quote, Report, TDReport10, TDReport15},
    tcb_info::TcbStatus,
    verify::{QuoteVerifier, VerifiedReport},
    QuoteCollateralV3,
};

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct PyQuoteCollateralV3 {
    inner: QuoteCollateralV3,
}

#[pymethods]
impl PyQuoteCollateralV3 {
    #[allow(clippy::too_many_arguments)]
    #[new]
    fn new(
        pck_crl_issuer_chain: String,
        root_ca_crl: Vec<u8>,
        pck_crl: Vec<u8>,
        tcb_info_issuer_chain: String,
        tcb_info: String,
        tcb_info_signature: Vec<u8>,
        qe_identity_issuer_chain: String,
        qe_identity: String,
        qe_identity_signature: Vec<u8>,
    ) -> Self {
        Self {
            inner: QuoteCollateralV3 {
                pck_crl_issuer_chain,
                root_ca_crl,
                pck_crl,
                tcb_info_issuer_chain,
                tcb_info,
                tcb_info_signature,
                qe_identity_issuer_chain,
                qe_identity,
                qe_identity_signature,
                pck_certificate_chain: None,
            },
        }
    }

    #[getter]
    fn pck_crl_issuer_chain(&self) -> &str {
        &self.inner.pck_crl_issuer_chain
    }

    #[getter]
    fn root_ca_crl(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.root_ca_crl).unbind()
    }

    #[getter]
    fn pck_crl(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.pck_crl).unbind()
    }

    #[getter]
    fn tcb_info_issuer_chain(&self) -> &str {
        &self.inner.tcb_info_issuer_chain
    }

    #[getter]
    fn tcb_info(&self) -> &str {
        &self.inner.tcb_info
    }

    #[getter]
    fn tcb_info_signature(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.tcb_info_signature).unbind()
    }

    #[getter]
    fn qe_identity_issuer_chain(&self) -> &str {
        &self.inner.qe_identity_issuer_chain
    }

    #[getter]
    fn qe_identity(&self) -> &str {
        &self.inner.qe_identity
    }

    #[getter]
    fn qe_identity_signature(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.qe_identity_signature).unbind()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize to JSON: {}", e)))
    }

    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: QuoteCollateralV3 = serde_json::from_str(json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;
        Ok(Self { inner })
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct PyVerifiedReport {
    inner: VerifiedReport,
}

#[pymethods]
impl PyVerifiedReport {
    #[getter]
    fn status(&self) -> &str {
        &self.inner.status
    }

    #[getter]
    fn advisory_ids(&self) -> Vec<String> {
        self.inner.advisory_ids.clone()
    }

    #[getter]
    fn ppid(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.ppid).unbind()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize to JSON: {}", e)))
    }
}

/// Quote header parsed from raw quote.
#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct PyQuoteHeader {
    inner: Header,
}

#[pymethods]
impl PyQuoteHeader {
    #[getter]
    fn version(&self) -> u16 {
        self.inner.version
    }

    #[getter]
    fn attestation_key_type(&self) -> u16 {
        self.inner.attestation_key_type
    }

    #[getter]
    fn tee_type(&self) -> u32 {
        self.inner.tee_type
    }

    #[getter]
    fn qe_svn(&self) -> u16 {
        self.inner.qe_svn
    }

    #[getter]
    fn pce_svn(&self) -> u16 {
        self.inner.pce_svn
    }

    #[getter]
    fn qe_vendor_id(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.qe_vendor_id).unbind()
    }

    #[getter]
    fn user_data(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.user_data).unbind()
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct PyTdReport10 {
    inner: TDReport10,
}

#[pymethods]
impl PyTdReport10 {
    #[getter]
    fn tee_tcb_svn(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.tee_tcb_svn).unbind()
    }
    #[getter]
    fn mr_seam(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_seam).unbind()
    }
    #[getter]
    fn mr_signer_seam(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_signer_seam).unbind()
    }
    #[getter]
    fn seam_attributes(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.seam_attributes).unbind()
    }
    #[getter]
    fn td_attributes(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.td_attributes).unbind()
    }
    #[getter]
    fn xfam(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.xfam).unbind()
    }
    #[getter]
    fn mr_td(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_td).unbind()
    }
    #[getter]
    fn mr_config_id(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_config_id).unbind()
    }
    #[getter]
    fn mr_owner(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_owner).unbind()
    }
    #[getter]
    fn mr_owner_config(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_owner_config).unbind()
    }
    #[getter]
    fn rt_mr0(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.rt_mr0).unbind()
    }
    #[getter]
    fn rt_mr1(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.rt_mr1).unbind()
    }
    #[getter]
    fn rt_mr2(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.rt_mr2).unbind()
    }
    #[getter]
    fn rt_mr3(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.rt_mr3).unbind()
    }
    #[getter]
    fn report_data(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.report_data).unbind()
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct PyTdReport15 {
    inner: TDReport15,
}

#[pymethods]
impl PyTdReport15 {
    // Flatten fields from base TDReport10
    #[getter]
    fn tee_tcb_svn(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.tee_tcb_svn).unbind()
    }
    #[getter]
    fn mr_seam(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.mr_seam).unbind()
    }
    #[getter]
    fn mr_signer_seam(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.mr_signer_seam).unbind()
    }
    #[getter]
    fn seam_attributes(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.seam_attributes).unbind()
    }
    #[getter]
    fn td_attributes(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.td_attributes).unbind()
    }
    #[getter]
    fn xfam(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.xfam).unbind()
    }
    #[getter]
    fn mr_td(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.mr_td).unbind()
    }
    #[getter]
    fn mr_config_id(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.mr_config_id).unbind()
    }
    #[getter]
    fn mr_owner(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.mr_owner).unbind()
    }
    #[getter]
    fn mr_owner_config(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.mr_owner_config).unbind()
    }
    #[getter]
    fn rt_mr0(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.rt_mr0).unbind()
    }
    #[getter]
    fn rt_mr1(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.rt_mr1).unbind()
    }
    #[getter]
    fn rt_mr2(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.rt_mr2).unbind()
    }
    #[getter]
    fn rt_mr3(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.rt_mr3).unbind()
    }
    #[getter]
    fn report_data(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.base.report_data).unbind()
    }

    // TD15 extra fields
    #[getter]
    fn tee_tcb_svn2(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.tee_tcb_svn2).unbind()
    }

    #[getter]
    fn mr_service_td(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_service_td).unbind()
    }
}

#[pyclass(from_py_object)]
#[derive(Clone, Copy)]
pub struct PySgxEnclaveReport {
    inner: EnclaveReport,
}

#[pymethods]
impl PySgxEnclaveReport {
    #[getter]
    fn cpu_svn(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.cpu_svn).unbind()
    }

    #[getter]
    fn attributes(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.attributes).unbind()
    }

    #[getter]
    fn mr_enclave(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_enclave).unbind()
    }

    #[getter]
    fn mr_signer(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.mr_signer).unbind()
    }

    #[getter]
    fn report_data(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.report_data).unbind()
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct PyPckExtension {
    inner: intel::PckExtension,
}

#[pymethods]
impl PyPckExtension {
    #[getter]
    fn ppid(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.ppid).unbind()
    }

    #[getter]
    fn cpu_svn(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.cpu_svn).unbind()
    }

    #[getter]
    fn pce_svn(&self) -> u16 {
        self.inner.pce_svn
    }

    #[getter]
    fn pce_id(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.pce_id).unbind()
    }

    #[getter]
    fn fmspc(&self, py: Python<'_>) -> Py<PyBytes> {
        PyBytes::new(py, &self.inner.fmspc).unbind()
    }

    #[getter]
    fn sgx_type(&self) -> u64 {
        self.inner.sgx_type
    }

    #[getter]
    fn platform_instance_id(&self, py: Python<'_>) -> Option<Py<PyBytes>> {
        self.inner
            .platform_instance_id
            .as_ref()
            .map(|v| PyBytes::new(py, v).unbind())
    }

    /// Look up an arbitrary OID inside the raw Intel SGX extension.
    fn get_value(&self, oid: &str, py: Python<'_>) -> PyResult<Option<Py<PyBytes>>> {
        let parsed_oid = const_oid::ObjectIdentifier::new(oid)
            .map_err(|e| PyValueError::new_err(format!("Invalid OID '{}': {}", oid, e)))?;
        match self.inner.get_value(&parsed_oid) {
            Ok(Some(bytes)) => Ok(Some(PyBytes::new(py, &bytes).unbind())),
            Ok(None) => Ok(None),
            Err(e) => Err(PyValueError::new_err(format!(
                "Failed to look up OID: {}",
                e
            ))),
        }
    }
}

#[pyclass(skip_from_py_object)]
pub struct PyQuote {
    inner: Quote,
}

#[pymethods]
impl PyQuote {
    #[staticmethod]
    fn parse(raw_quote: &Bound<'_, PyBytes>) -> PyResult<Self> {
        let quote_bytes = raw_quote.as_bytes();
        match Quote::parse(quote_bytes) {
            Ok(quote) => Ok(PyQuote { inner: quote }),
            Err(e) => Err(PyValueError::new_err(format!(
                "Failed to parse quote: {}",
                e
            ))),
        }
    }

    /// Parsed quote header.
    #[getter]
    fn header(&self) -> PyQuoteHeader {
        PyQuoteHeader {
            inner: self.inner.header,
        }
    }

    /// Parsed quote report (TDX TDREPORT10/15 or SGX enclave report).
    #[getter]
    fn report<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        match &self.inner.report {
            Report::SgxEnclave(r) => Ok(Py::new(py, PySgxEnclaveReport { inner: *r })?
                .into_pyobject(py)
                .unwrap()
                .into_any()),
            Report::TD10(r) => Ok(Py::new(py, PyTdReport10 { inner: *r })?
                .into_pyobject(py)
                .unwrap()
                .into_any()),
            Report::TD15(r) => Ok(Py::new(py, PyTdReport15 { inner: *r })?
                .into_pyobject(py)
                .unwrap()
                .into_any()),
        }
    }

    fn fmspc(&self) -> PyResult<String> {
        match intel::quote_fmspc(&self.inner) {
            Ok(fmspc) => Ok(hex::encode_upper(fmspc)),
            Err(e) => Err(PyValueError::new_err(format!("Failed to get FMSPC: {}", e))),
        }
    }

    fn ca(&self) -> PyResult<String> {
        match intel::quote_ca(&self.inner) {
            Ok(ca) => Ok(ca.as_id_str().to_string()),
            Err(e) => Err(PyValueError::new_err(format!("Failed to get CA: {}", e))),
        }
    }

    fn is_tdx(&self) -> bool {
        !self.is_sgx()
    }

    fn is_sgx(&self) -> bool {
        self.inner.header.is_sgx()
    }

    fn quote_type(&self) -> String {
        if self.inner.header.is_sgx() {
            "SGX".to_string()
        } else {
            "TDX".to_string()
        }
    }

    /// Get the embedded PCK certificate chain in PEM form, if present.
    ///
    /// Returns bytes instead of str to avoid implicit decoding/copying.
    fn cert_chain_pem_bytes(&self, py: Python<'_>) -> PyResult<Option<Py<PyBytes>>> {
        let raw = match self.inner.raw_cert_chain() {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let mut trimmed = raw;
        while let Some(without_suffix) = trimmed.strip_suffix(&[0]) {
            trimmed = without_suffix;
        }
        Ok(Some(PyBytes::new(py, trimmed).unbind()))
    }

    /// Parse the Intel SGX extension from the leaf PCK certificate.
    ///
    /// Returns None if the quote does not contain a parseable leaf certificate.
    fn pck_extension(&self) -> PyResult<Option<PyPckExtension>> {
        let certs = match intel::extract_cert_chain(&self.inner) {
            Ok(certs) => certs,
            Err(_) => return Ok(None),
        };
        let leaf = match certs.first() {
            Some(c) => c,
            None => return Ok(None),
        };
        match intel::parse_pck_extension(leaf) {
            Ok(ext) => Ok(Some(PyPckExtension { inner: ext })),
            Err(_) => Ok(None),
        }
    }
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct PyQuotePolicy {
    inner: QuotePolicy,
}

fn parse_tcb_status(value: &str) -> PyResult<TcbStatus> {
    match value {
        "UpToDate" => Ok(TcbStatus::UpToDate),
        "SWHardeningNeeded" => Ok(TcbStatus::SWHardeningNeeded),
        "ConfigurationNeeded" => Ok(TcbStatus::ConfigurationNeeded),
        "ConfigurationAndSWHardeningNeeded" => Ok(TcbStatus::ConfigurationAndSWHardeningNeeded),
        "OutOfDate" => Ok(TcbStatus::OutOfDate),
        "OutOfDateConfigurationNeeded" => Ok(TcbStatus::OutOfDateConfigurationNeeded),
        "Revoked" => Ok(TcbStatus::Revoked),
        _ => Err(PyValueError::new_err(format!(
            "Unknown TCB status: {value}"
        ))),
    }
}

#[pymethods]
impl PyQuotePolicy {
    #[staticmethod]
    fn strict(now_secs: u64) -> Self {
        Self {
            inner: QuotePolicy::strict(now_secs),
        }
    }

    #[staticmethod]
    fn claims_only(now_secs: u64) -> Self {
        Self {
            inner: QuotePolicy::claims_only(now_secs),
        }
    }
    fn allow_status(&self, status: &str) -> PyResult<Self> {
        Ok(Self {
            inner: self.inner.clone().allow_status(parse_tcb_status(status)?),
        })
    }
    fn reject_advisory(&self, id: &str) -> Self {
        Self {
            inner: self.inner.clone().reject_advisory(id),
        }
    }
    fn reject_advisories(&self, ids: Vec<String>) -> Self {
        Self {
            inner: self.inner.clone().reject_advisories(&ids),
        }
    }
    fn platform_grace_period(&self, secs: u64) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .platform_grace_period(Duration::from_secs(secs)),
        }
    }
    fn qe_grace_period(&self, secs: u64) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .qe_grace_period(Duration::from_secs(secs)),
        }
    }
    fn min_tcb_eval_data_number(&self, value: u32) -> Self {
        Self {
            inner: self.inner.clone().min_tcb_eval_data_number(value),
        }
    }
    fn allow_dynamic_platform(&self, value: bool) -> Self {
        Self {
            inner: self.inner.clone().allow_dynamic_platform(value),
        }
    }
    fn allow_cached_keys(&self, value: bool) -> Self {
        Self {
            inner: self.inner.clone().allow_cached_keys(value),
        }
    }
    fn allow_smt(&self, value: bool) -> Self {
        Self {
            inner: self.inner.clone().allow_smt(value),
        }
    }
    fn accepted_sgx_types(&self, values: Vec<u8>) -> Self {
        Self {
            inner: self.inner.clone().accepted_sgx_types(&values),
        }
    }
}

#[pyclass]
pub struct PyQuoteClaims {
    inner: QuoteClaims,
}

#[pymethods]
impl PyQuoteClaims {
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize claims: {e}")))
    }
}

#[pyclass]
pub struct PyQuoteVerifier {
    inner: QuoteVerifier,
}

#[pymethods]
impl PyQuoteVerifier {
    #[new]
    #[pyo3(signature = (root_ca_der=None))]
    fn new(root_ca_der: Option<Vec<u8>>) -> Self {
        let inner = root_ca_der.map_or_else(QuoteVerifier::new_prod, QuoteVerifier::new);
        Self { inner }
    }

    fn verify_with_policy(
        &self,
        raw_quote: &Bound<'_, PyBytes>,
        collateral: &PyQuoteCollateralV3,
        now_secs: u64,
        policy: &PyQuotePolicy,
    ) -> PyResult<PyQuoteClaims> {
        self.inner
            .verify_with_policy(
                raw_quote.as_bytes(),
                collateral.inner.clone(),
                now_secs,
                &policy.inner,
            )
            .map(|inner| PyQuoteClaims { inner })
            .map_err(|e| PyValueError::new_err(format!("Verification failed: {e}")))
    }
}

#[pyfunction]
fn py_verify(
    raw_quote: &Bound<'_, PyBytes>,
    collateral: &PyQuoteCollateralV3,
    now_secs: u64,
) -> PyResult<PyVerifiedReport> {
    let quote_bytes = raw_quote.as_bytes();

    QuoteVerifier::new_prod()
        .verify(quote_bytes, &collateral.inner, now_secs)
        .map(|inner| PyVerifiedReport { inner })
        .map_err(|e| PyValueError::new_err(format!("Verification failed: {e:?}")))
}

#[pyfunction]
fn py_verify_with_root_ca(
    raw_quote: &Bound<'_, PyBytes>,
    collateral: &PyQuoteCollateralV3,
    root_ca_der: &Bound<'_, PyBytes>,
    now_secs: u64,
) -> PyResult<PyVerifiedReport> {
    let quote_bytes = raw_quote.as_bytes();
    let root_ca = root_ca_der.as_bytes();

    let verifier = crate::verify::QuoteVerifier::new(root_ca.to_vec());
    verifier
        .verify(quote_bytes, &collateral.inner, now_secs)
        .map(|inner| PyVerifiedReport { inner })
        .map_err(|e| PyValueError::new_err(format!("Verification failed: {e:?}")))
}

#[pyfunction]
fn parse_quote(raw_quote: &Bound<'_, PyBytes>) -> PyResult<PyQuote> {
    PyQuote::parse(raw_quote)
}

#[pyfunction]
fn parse_pck_extension_from_pem(pem_bytes: &Bound<'_, PyBytes>) -> PyResult<PyPckExtension> {
    let pem_data = pem_bytes.as_bytes();
    match intel::parse_pck_extension_from_pem(pem_data) {
        Ok(ext) => Ok(PyPckExtension { inner: ext }),
        Err(e) => Err(PyValueError::new_err(format!(
            "Failed to parse PCK extension: {}",
            e
        ))),
    }
}

#[pyfunction(name = "get_collateral")]
fn get_collateral_py<'py>(
    py: Python<'py>,
    pccs_url: String,
    raw_quote: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let raw_quote = raw_quote
        .cast::<PyBytes>()
        .map_err(|_| PyTypeError::new_err("raw_quote must be bytes"))?
        .as_bytes()
        .to_vec();
    future_into_py(py, async move {
        let client = CollateralClient::with_default_http(pccs_url)
            .map_err(|e| PyValueError::new_err(format!("Failed to build HTTP client: {}", e)))?;
        let collateral = client
            .fetch(&raw_quote)
            .await
            .map_err(|e| PyValueError::new_err(format!("Failed to get collateral: {}", e)))?;
        Ok(PyQuoteCollateralV3 { inner: collateral })
    })
}

pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQuoteCollateralV3>()?;
    m.add_class::<PyVerifiedReport>()?;
    m.add_class::<PyQuoteHeader>()?;
    m.add_class::<PyTdReport10>()?;
    m.add_class::<PyTdReport15>()?;
    m.add_class::<PySgxEnclaveReport>()?;
    m.add_class::<PyPckExtension>()?;
    m.add_class::<PyQuote>()?;
    m.add_class::<PyQuotePolicy>()?;
    m.add_class::<PyQuoteClaims>()?;
    m.add_class::<PyQuoteVerifier>()?;
    m.add_function(wrap_pyfunction!(py_verify, m)?)?;
    m.add_function(wrap_pyfunction!(py_verify_with_root_ca, m)?)?;
    m.add_function(wrap_pyfunction!(parse_quote, m)?)?;
    m.add_function(wrap_pyfunction!(parse_pck_extension_from_pem, m)?)?;
    m.add_function(wrap_pyfunction!(get_collateral_py, m)?)?;

    Ok(())
}
