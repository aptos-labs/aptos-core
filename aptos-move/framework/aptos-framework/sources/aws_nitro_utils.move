// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Utilities for verifying AWS Nitro Enclave attestation documents.
module aptos_framework::aws_nitro_utils {
    use std::option::{Self, Option};
    use std::vector;

    /// One Platform Configuration Register (PCR) entry from the attestation document.
    struct PcrEntry has copy, drop, store {
        index: u8,
        value: vector<u8>,
    }

    /// Parsed attestation document fields (when validation succeeds).
    struct AttestationDoc has copy, drop, store {
        /// NitroSecureModule identifier (e.g. NSM module ID).
        module_id: vector<u8>,
        /// Unix timestamp in milliseconds when the document was created.
        timestamp: u64,
        /// Digest algorithm identifier (e.g. b"SHA384").
        digest: vector<u8>,
        /// Platform Configuration Registers: index and value pairs.
        pcrs: vector<PcrEntry>,
        /// Signing certificate (DER-encoded).
        certificate: vector<u8>,
        /// User-provided data, if present.
        user_data: Option<vector<u8>>,
        /// Nonce, if present.
        nonce: Option<vector<u8>>,
        /// DER-encoded public key, if present.
        public_key: Option<vector<u8>>,
    }

    /// Attestation documents are produced by the AWS Nitro Enclaves NSM (Nitro Security Module)
    /// in COSE Sign1 format. This module provides natives that verify and parse such documents.
    ///
    /// Example (in Move):
    /// ```move
    /// use aptos_framework::aws_nitro_utils;
    /// use std::option;
    ///
    /// fun check_attestation(attestation_doc: vector<u8>) {
    ///     assert!(aws_nitro_utils::verify_attestation(attestation_doc), 0);
    /// }
    ///
    /// fun parse_and_use(attestation_doc: vector<u8>) {
    ///     let doc_opt = aws_nitro_utils::verify_and_parse_attestation(attestation_doc);
    ///     if (option::is_some(&doc_opt)) {
    ///         let doc = option::extract(&mut doc_opt);
    ///         // use doc.module_id, doc.timestamp, doc.pcrs, etc.
    ///     }
    /// }
    /// ```

    /// Verifies an AWS Nitro Enclave attestation document.
    ///
    /// The input must be the raw bytes of a COSE Sign1-encoded attestation document
    /// as returned by the Nitro Enclave NSM (e.g. from GetAttestationDocument).
    ///
    /// Returns `true` if the document is valid; `false` otherwise.
    public native fun verify_attestation(attestation_doc: vector<u8>): bool;

    /// Verifies and parses an AWS Nitro Enclave attestation document.
    ///
    /// Same validation as `verify_attestation`. On success returns `Some(AttestationDoc)` with
    /// decoded fields (module_id, timestamp, digest, pcrs, certificate, user_data, nonce, public_key).
    /// Returns `None` if the document is invalid or malformed.
    public native fun verify_and_parse_attestation(attestation_doc: vector<u8>): Option<AttestationDoc>;

    #[test_only]
    fun test_invalid_attestation_returns_false() {
        // Empty input is not valid COSE Sign1
        assert!(!verify_attestation(vector::empty<u8>()), 0);
    }

    #[test_only]
    fun test_garbage_bytes_return_false() {
        // Random bytes are not a valid attestation document
        let garbage = vector[
            0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x06u8, 0x07u8,
            0x08u8, 0x09u8, 0x0au8, 0x0bu8, 0x0cu8, 0x0du8, 0x0eu8, 0x0fu8,
        ];
        assert!(!verify_attestation(garbage), 0);
    }

    #[test_only]
    fun test_verify_and_parse_invalid_returns_none() {
        let doc_opt = verify_and_parse_attestation(vector::empty<u8>());
        assert!(option::is_none(&doc_opt), 0);
    }
}
