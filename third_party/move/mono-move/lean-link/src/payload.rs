// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Deterministic JSON payload v1 crossing the Lean/MonoVM boundary.
//!
//! Every type here derives its JSON shape, and the shapes are the contract:
//! field order follows declaration order, maps are `BTreeMap`s, and integers
//! travel as decimal strings so every width round trips exactly. The
//! top-level `version` is the payload schema version and is independent of
//! the native ABI version of the C boundary.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version of the JSON payload schema.
pub const PAYLOAD_VERSION: u32 = 1;

/// One Move source file of the request's compilation unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
}

/// The compilation unit: sources, named addresses, and language version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompileSpec {
    pub sources: Vec<SourceFile>,
    /// Named-address assignments on top of the Move stdlib defaults, as
    /// `name -> 0x…` hex literals. Assigning a stdlib name again lets the
    /// request override that default.
    #[serde(default)]
    pub addresses: BTreeMap<String, String>,
    /// Move language version; `2` selects the latest stable version.
    pub language: u64,
}

/// Resource limits for every call of the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Finite gas budget per call. MonoMove gas units are uncalibrated, so
    /// this is a termination bound and a diagnostic, never a compared
    /// quantity.
    pub gas: u64,
    /// Optional heap size in bytes per call, bounding allocation before
    /// exhaustion is reported.
    #[serde(default)]
    pub heap: Option<usize>,
}

/// One function call of the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    /// The callee as `0x…::module::function`.
    pub function: String,
    /// Signer addresses for `signer`/`&signer` parameters, in parameter
    /// order.
    #[serde(default)]
    pub signers: Vec<String>,
    /// Typed arguments in declaration order, excluding signer parameters.
    pub args: Vec<Value>,
}

/// A typed value in the stable recursive schema. The adapter, not Lean, is
/// responsible for translating between this schema and MonoMove's BCS
/// argument and result representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Value {
    #[serde(rename = "unit")]
    Unit,
    #[serde(rename = "bool")]
    Bool { value: bool },
    /// A fixed-width integer; `value` is a decimal string, two's complement
    /// for signed widths.
    #[serde(rename = "integer")]
    Integer {
        width: u16,
        signed: bool,
        value: String,
    },
    /// An address as a canonical `0x…` hex literal.
    #[serde(rename = "address")]
    Address { value: String },
    /// A homogeneous vector; `elements` of equal encoded width.
    #[serde(rename = "vector")]
    Vector { elements: Vec<Value> },
}

/// The adapter's build identity, echoed in every response so the Lean
/// harness can log it beside its Move frontend's identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub abi: u32,
    pub rustc: String,
    pub profile: String,
}

/// The resource that ran out before a call completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExhaustedResource {
    Gas,
    Heap,
}

/// The stage that produced an `error` outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Compile,
    Load,
    Run,
    Abi,
    Internal,
}

/// The normalized outcome of one request call. Abort locations, messages,
/// error stages, and diagnostic text are carried for triage only and are
/// never part of semantic equality.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Outcome {
    #[serde(rename = "returned")]
    Returned {
        values: Vec<Value>,
        gas_used: u64,
        gc_count: usize,
    },
    #[serde(rename = "aborted")]
    Aborted {
        code: u64,
        location: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "exhausted")]
    Exhausted { resource: ExhaustedResource },
    /// The program failed at runtime: MonoVM raised a typed execution error
    /// rather than aborting. This is an outcome *of the program*, not of the
    /// harness — arithmetic overflow, a vector index out of bounds and the
    /// like reach the caller this way. `kind` is MonoVM's
    /// `ExecutionErrorKind`, which is what callers branch on; `message` is
    /// human-readable diagnostic text that must not be parsed.
    #[serde(rename = "failed")]
    Failed { failure: String, message: String },
    /// The harness could not obtain an outcome at all.
    #[serde(rename = "error")]
    Error { stage: Stage, message: String },
}

/// One linked execution request: compile once, then run every call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub version: u32,
    pub compile: CompileSpec,
    pub limits: Limits,
    pub calls: Vec<Call>,
}

/// The response to a request: one outcome per call, in request order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub version: u32,
    pub identity: Identity,
    pub outcomes: Vec<Outcome>,
}

impl Response {
    /// A response reporting the same failure for every call of `request`.
    pub fn uniform_error(
        request: &Request,
        identity: &Identity,
        stage: Stage,
        message: String,
    ) -> Self {
        Self {
            version: PAYLOAD_VERSION,
            identity: identity.clone(),
            outcomes: request
                .calls
                .iter()
                .map(|_| Outcome::Error {
                    stage,
                    message: message.clone(),
                })
                .collect(),
        }
    }
}
