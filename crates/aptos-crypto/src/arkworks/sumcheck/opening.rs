// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Minimal opening accumulators (stubs for sumcheck; no PCS).

use crate::arkworks::sumcheck::field::SumcheckField;

/// Prover-side accumulator; no-op for our use.
#[derive(Clone, Debug)]
pub struct ProverOpeningAccumulator<F: SumcheckField> {
    _log_t: usize,
    _marker: std::marker::PhantomData<F>,
}

impl<F: SumcheckField> ProverOpeningAccumulator<F> {
    pub fn new(log_t: usize) -> Self {
        Self {
            _log_t: log_t,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn flush_to_transcript<T>(&self, _transcript: &mut T) {
        // No-op
    }
}

/// Verifier-side accumulator; no-op for our use.
#[derive(Clone, Debug)]
pub struct VerifierOpeningAccumulator<F: SumcheckField> {
    _log_t: usize,
    _marker: std::marker::PhantomData<F>,
}

impl<F: SumcheckField> VerifierOpeningAccumulator<F> {
    pub fn new(log_t: usize, _zk_mode: bool) -> Self {
        Self {
            _log_t: log_t,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn flush_to_transcript<T>(&self, _transcript: &mut T) {
        // No-op
    }
}
