// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Utilities for verifying AWS Nitro Enclave attestation documents.
module aptos_framework::aws_nitro_utils {
    use aptos_framework::aptos_governance;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use std::error;
    use std::option;
    use std::option::Option;

    #[test_only]
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

    /// Governance-managed DER-encoded AWS Nitro root certificates trusted by this chain.
    struct TrustedRoots has key {
        root_certs: vector<vector<u8>>,
    }

    const ETRUSTED_ROOTS_ALREADY_INITIALIZED: u64 = 1;
    const ETRUSTED_ROOTS_NOT_INITIALIZED: u64 = 2;
    const ETRUSTED_ROOT_ALREADY_EXISTS: u64 = 3;
    const ETRUSTED_ROOT_NOT_FOUND: u64 = 4;

    /// Attestation documents are produced by the AWS Nitro Enclaves NSM (Nitro Security Module)
    /// in COSE Sign1 format. This module provides natives that verify and parse such documents.
    ///
    /// Example (in Move):
    /// ```move
    /// use aptos_framework::aws_nitro_utils;
    ///
    /// fun check_attestation(attestation_doc: vector<u8>) {
    ///     assert!(aws_nitro_utils::verify_attestation(attestation_doc), 0);
    /// }
    ///
    /// fun parse_and_use(attestation_doc: vector<u8>) {
    ///     let doc_opt = aws_nitro_utils::verify_and_parse_attestation(attestation_doc);
    ///     if (doc_opt.is_some()) {
    ///         let doc = doc_opt.extract();
    ///         let nonce = aws_nitro_utils::nonce(&doc);
    ///         let pcr0 = aws_nitro_utils::pcr(&doc, 0);
    ///         // Check nonce, PCRs, user_data, public_key, etc.
    ///     }
    /// }
    /// ```

    /// Verifies an AWS Nitro Enclave attestation document using chain-managed
    /// Nitro roots and consensus time.
    ///
    /// The input must be the raw bytes of a COSE Sign1-encoded attestation document
    /// as returned by the Nitro Enclave NSM (e.g. from GetAttestationDocument).
    ///
    /// Returns `true` if the document is valid; `false` otherwise. Returns `false`
    /// if the chain-managed root store has not been initialized.
    public fun verify_attestation(attestation_doc: vector<u8>): bool {
        if (!is_initialized()) {
            return false
        };
        verify_attestation_from_store(attestation_doc, timestamp::now_seconds())
    }

    /// Verifies and parses an AWS Nitro Enclave attestation document using
    /// chain-managed Nitro roots and consensus time.
    ///
    /// Same validation as `verify_attestation`. On success returns `Some(AttestationDoc)` with
    /// decoded fields (module_id, timestamp, digest, pcrs, certificate, user_data, nonce, public_key).
    /// Returns `None` if the document is invalid, malformed, or if the chain-managed
    /// root store has not been initialized.
    public fun verify_and_parse_attestation(attestation_doc: vector<u8>): Option<AttestationDoc> {
        if (!is_initialized()) {
            return option::none()
        };
        verify_and_parse_attestation_from_store(attestation_doc, timestamp::now_seconds())
    }

    /// Verifies an AWS Nitro Enclave attestation document and checks that its
    /// `user_data` field equals `expected_user_data`.
    ///
    /// This is a small dapp-facing helper for policies that only need to bind
    /// the attested enclave to an application-specific request or account.
    public fun verify_attestation_user_data(
        attestation_doc: vector<u8>,
        expected_user_data: &vector<u8>,
    ): bool {
        if (!is_initialized()) {
            return false
        };
        verify_attestation_user_data_from_store(
            attestation_doc,
            timestamp::now_seconds(),
            expected_user_data,
        )
    }

    /// Verifies an AWS Nitro Enclave attestation document against caller-provided
    /// DER-encoded root certificates.
    ///
    /// `unix_time_secs` is used for certificate validity checks. Callers should pass
    /// consensus time, not local wall-clock time.
    public native fun verify_attestation_with_roots(
        attestation_doc: vector<u8>,
        trusted_root_certs: vector<vector<u8>>,
        unix_time_secs: u64,
    ): bool;

    /// Verifies and parses an AWS Nitro Enclave attestation document against
    /// caller-provided DER-encoded root certificates.
    public native fun verify_and_parse_attestation_with_roots(
        attestation_doc: vector<u8>,
        trusted_root_certs: vector<vector<u8>>,
        unix_time_secs: u64,
    ): Option<AttestationDoc>;

    /// Initializes the chain-managed Nitro root certificate store.
    public entry fun initialize(aptos_framework: &signer, root_certs: vector<vector<u8>>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            !exists<TrustedRoots>(@aptos_framework),
            error::already_exists(ETRUSTED_ROOTS_ALREADY_INITIALIZED),
        );
        move_to(aptos_framework, TrustedRoots { root_certs });
    }

    /// Initializes the Nitro root certificate store from the core resources account.
    /// This is intended for localnet/testnet flows where core resources can mint APT and
    /// has governance signer capabilities.
    public entry fun initialize_testnet_only(core_resources: &signer, root_certs: vector<vector<u8>>) {
        let aptos_framework = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        initialize(&aptos_framework, root_certs);
    }

    #[view]
    /// Returns true iff the chain-managed Nitro root certificate store exists.
    public fun is_initialized(): bool {
        exists<TrustedRoots>(@aptos_framework)
    }

    fun assert_trusted_roots_initialized() {
        assert!(
            exists<TrustedRoots>(@aptos_framework),
            error::not_found(ETRUSTED_ROOTS_NOT_INITIALIZED),
        );
    }

    /// Replaces the chain-managed Nitro root certificate set.
    public entry fun set_trusted_roots(aptos_framework: &signer, root_certs: vector<vector<u8>>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert_trusted_roots_initialized();

        let trusted_roots = &mut TrustedRoots[@aptos_framework];
        trusted_roots.root_certs = root_certs;
    }

    /// Replaces the Nitro root certificate set from the core resources account.
    /// Intended for localnet/testnet root rotation drills.
    public entry fun set_trusted_roots_testnet_only(core_resources: &signer, root_certs: vector<vector<u8>>) {
        let aptos_framework = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        set_trusted_roots(&aptos_framework, root_certs);
    }

    /// Adds one DER-encoded Nitro root certificate to the chain-managed set.
    public entry fun add_trusted_root(aptos_framework: &signer, root_cert: vector<u8>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert_trusted_roots_initialized();

        let trusted_roots = &mut TrustedRoots[@aptos_framework];
        assert!(
            !trusted_roots.root_certs.contains(&root_cert),
            error::already_exists(ETRUSTED_ROOT_ALREADY_EXISTS),
        );
        trusted_roots.root_certs.push_back(root_cert);
    }

    /// Adds one Nitro root certificate from the core resources account.
    /// Intended for localnet/testnet root rotation drills.
    public entry fun add_trusted_root_testnet_only(core_resources: &signer, root_cert: vector<u8>) {
        let aptos_framework = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        add_trusted_root(&aptos_framework, root_cert);
    }

    /// Removes one DER-encoded Nitro root certificate from the chain-managed set.
    public entry fun remove_trusted_root(aptos_framework: &signer, root_cert: vector<u8>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert_trusted_roots_initialized();

        let root_certs = &mut TrustedRoots[@aptos_framework].root_certs;
        let len = root_certs.length();
        let i = 0;
        while (i < len) {
            if (root_certs.borrow(i) == &root_cert) {
                root_certs.remove(i);
                return
            };
            i = i + 1;
        };
        abort error::not_found(ETRUSTED_ROOT_NOT_FOUND)
    }

    /// Removes one Nitro root certificate from the core resources account.
    /// Intended for localnet/testnet root rotation drills.
    public entry fun remove_trusted_root_testnet_only(core_resources: &signer, root_cert: vector<u8>) {
        let aptos_framework = aptos_governance::get_signer_testnet_only(core_resources, @aptos_framework);
        remove_trusted_root(&aptos_framework, root_cert);
    }

    #[view]
    /// Returns a copy of the chain-managed DER-encoded Nitro root certificates.
    public fun trusted_roots(): vector<vector<u8>> {
        assert_trusted_roots_initialized();
        TrustedRoots[@aptos_framework].root_certs
    }

    #[view]
    /// Returns the number of chain-managed Nitro root certificates.
    public fun trusted_root_count(): u64 {
        assert_trusted_roots_initialized();
        TrustedRoots[@aptos_framework].root_certs.length()
    }

    /// Returns true iff `root_cert` is currently trusted by the chain-managed store.
    public fun contains_trusted_root(root_cert: &vector<u8>): bool {
        if (!exists<TrustedRoots>(@aptos_framework)) {
            return false
        };
        TrustedRoots[@aptos_framework].root_certs.contains(root_cert)
    }

    /// Verifies an attestation using the chain-managed Nitro root certificates.
    public fun verify_attestation_from_store(attestation_doc: vector<u8>, unix_time_secs: u64): bool {
        if (!is_initialized()) {
            return false
        };
        verify_attestation_with_roots(
            attestation_doc,
            TrustedRoots[@aptos_framework].root_certs,
            unix_time_secs,
        )
    }

    /// Verifies and parses an attestation using the chain-managed Nitro root certificates.
    public fun verify_and_parse_attestation_from_store(
        attestation_doc: vector<u8>,
        unix_time_secs: u64,
    ): Option<AttestationDoc> {
        if (!is_initialized()) {
            return option::none()
        };
        verify_and_parse_attestation_with_roots(
            attestation_doc,
            TrustedRoots[@aptos_framework].root_certs,
            unix_time_secs,
        )
    }

    /// Verifies an attestation using the chain-managed Nitro root certificates
    /// and checks that its `user_data` field equals `expected_user_data`.
    public fun verify_attestation_user_data_from_store(
        attestation_doc: vector<u8>,
        unix_time_secs: u64,
        expected_user_data: &vector<u8>,
    ): bool {
        if (!is_initialized()) {
            return false
        };
        verify_attestation_user_data_with_roots(
            attestation_doc,
            TrustedRoots[@aptos_framework].root_certs,
            unix_time_secs,
            expected_user_data,
        )
    }

    /// Verifies an attestation against caller-provided DER-encoded root
    /// certificates and checks that its `user_data` field equals
    /// `expected_user_data`.
    public fun verify_attestation_user_data_with_roots(
        attestation_doc: vector<u8>,
        trusted_root_certs: vector<vector<u8>>,
        unix_time_secs: u64,
        expected_user_data: &vector<u8>,
    ): bool {
        let doc_opt = verify_and_parse_attestation_with_roots(
            attestation_doc,
            trusted_root_certs,
            unix_time_secs,
        );
        if (doc_opt.is_none()) {
            return false
        };

        let doc = doc_opt.extract();
        user_data_equals(&doc, expected_user_data)
    }

    /// Convenience verifier for key-release policies that bind a Nitro attestation
    /// to one PCR, nonce, user data, and public key.
    ///
    /// For more complex policies, call `verify_and_parse_attestation` and combine
    /// the accessors below. In ACE-style key release, `expected_public_key` should
    /// be the requester key that released key shares will be encrypted to, and
    /// `expected_user_data` should be a domain-separated hash of the full request
    /// context, such as `(enc_pk, job_id, label_hash, input_hash, nonce)`.
    public fun verify_attestation_matches(
        attestation_doc: vector<u8>,
        pcr_index: u8,
        expected_pcr: &vector<u8>,
        expected_nonce: &vector<u8>,
        expected_user_data: &vector<u8>,
        expected_public_key: &vector<u8>,
    ): bool {
        let doc_opt = verify_and_parse_attestation(attestation_doc);
        if (doc_opt.is_none()) {
            return false
        };

        let doc = doc_opt.extract();
        pcr_equals(&doc, pcr_index, expected_pcr)
            && nonce_equals(&doc, expected_nonce)
            && user_data_equals(&doc, expected_user_data)
            && public_key_equals(&doc, expected_public_key)
    }

    /// Same policy helper as `verify_attestation_matches`, but validates against
    /// caller-provided DER-encoded root certificates and explicit consensus time.
    public fun verify_attestation_matches_with_roots(
        attestation_doc: vector<u8>,
        trusted_root_certs: vector<vector<u8>>,
        unix_time_secs: u64,
        pcr_index: u8,
        expected_pcr: &vector<u8>,
        expected_nonce: &vector<u8>,
        expected_user_data: &vector<u8>,
        expected_public_key: &vector<u8>,
    ): bool {
        let doc_opt = verify_and_parse_attestation_with_roots(
            attestation_doc,
            trusted_root_certs,
            unix_time_secs,
        );
        if (doc_opt.is_none()) {
            return false
        };

        let doc = doc_opt.extract();
        pcr_equals(&doc, pcr_index, expected_pcr)
            && nonce_equals(&doc, expected_nonce)
            && user_data_equals(&doc, expected_user_data)
            && public_key_equals(&doc, expected_public_key)
    }

    /// Same policy helper as `verify_attestation_matches`, but validates against
    /// the chain-managed root certificate store.
    public fun verify_attestation_matches_from_store(
        attestation_doc: vector<u8>,
        unix_time_secs: u64,
        pcr_index: u8,
        expected_pcr: &vector<u8>,
        expected_nonce: &vector<u8>,
        expected_user_data: &vector<u8>,
        expected_public_key: &vector<u8>,
    ): bool {
        if (!is_initialized()) {
            return false
        };
        verify_attestation_matches_with_roots(
            attestation_doc,
            TrustedRoots[@aptos_framework].root_certs,
            unix_time_secs,
            pcr_index,
            expected_pcr,
            expected_nonce,
            expected_user_data,
            expected_public_key,
        )
    }

    /// Returns the NitroSecureModule identifier.
    public fun module_id(doc: &AttestationDoc): vector<u8> {
        doc.module_id
    }

    /// Returns the document creation timestamp, in Unix milliseconds.
    public fun timestamp(doc: &AttestationDoc): u64 {
        doc.timestamp
    }

    /// Returns the digest algorithm identifier parsed from the document.
    public fun digest(doc: &AttestationDoc): vector<u8> {
        doc.digest
    }

    /// Returns the signing certificate, DER-encoded.
    public fun certificate(doc: &AttestationDoc): vector<u8> {
        doc.certificate
    }

    /// Returns the optional user-provided data.
    public fun user_data(doc: &AttestationDoc): Option<vector<u8>> {
        doc.user_data
    }

    /// Returns the optional nonce.
    public fun nonce(doc: &AttestationDoc): Option<vector<u8>> {
        doc.nonce
    }

    /// Returns the optional DER-encoded public key.
    public fun public_key(doc: &AttestationDoc): Option<vector<u8>> {
        doc.public_key
    }

    /// Returns the number of PCR entries in the document.
    public fun pcr_count(doc: &AttestationDoc): u64 {
        doc.pcrs.length()
    }

    /// Returns the PCR entry at `offset` as `(index, value)`.
    public fun pcr_at(doc: &AttestationDoc, offset: u64): (u8, vector<u8>) {
        let entry = doc.pcrs.borrow(offset);
        (entry.index, entry.value)
    }

    /// Returns the PCR value with the requested `index`, if present.
    public fun pcr(doc: &AttestationDoc, index: u8): Option<vector<u8>> {
        let len = doc.pcrs.length();
        let offset = 0;
        while (offset < len) {
            let entry = doc.pcrs.borrow(offset);
            if (entry.index == index) {
                return option::some(entry.value)
            };
            offset = offset + 1;
        };
        option::none()
    }

    /// Returns true iff the requested PCR is present and equals `expected`.
    public fun pcr_equals(doc: &AttestationDoc, index: u8, expected: &vector<u8>): bool {
        let pcr_opt = pcr(doc, index);
        pcr_opt.is_some() && pcr_opt.borrow() == expected
    }

    /// Returns true iff the attestation's `user_data` is present and equals `expected`.
    ///
    /// Dapps should bind `user_data` to their own domain-separated hash, for example
    /// `hash(enc_pk || job_id || input_hash || nonce)`.
    public fun user_data_equals(doc: &AttestationDoc, expected: &vector<u8>): bool {
        doc.user_data.is_some() && doc.user_data.borrow() == expected
    }

    /// Returns true iff the attestation's `nonce` is present and equals `expected`.
    public fun nonce_equals(doc: &AttestationDoc, expected: &vector<u8>): bool {
        doc.nonce.is_some() && doc.nonce.borrow() == expected
    }

    /// Returns true iff the attestation's `public_key` is present and equals `expected`.
    public fun public_key_equals(doc: &AttestationDoc, expected: &vector<u8>): bool {
        doc.public_key.is_some() && doc.public_key.borrow() == expected
    }

    #[test]
    fun test_invalid_attestation_returns_false() {
        // Empty input is not valid COSE Sign1
        assert!(!verify_attestation(vector::empty<u8>()), 0);
        assert!(
            !verify_attestation_with_roots(vector::empty<u8>(), vector[b"fake-root"], 0),
            1,
        );
    }

    #[test]
    fun test_garbage_bytes_return_false() {
        // Random bytes are not a valid attestation document
        let garbage = vector[
            0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x06u8, 0x07u8,
            0x08u8, 0x09u8, 0x0au8, 0x0bu8, 0x0cu8, 0x0du8, 0x0eu8, 0x0fu8,
        ];
        assert!(!verify_attestation(copy garbage), 0);
        assert!(
            !verify_attestation_with_roots(garbage, vector[b"fake-root"], 0),
            1,
        );
    }

    #[test]
    fun test_verify_and_parse_invalid_returns_none() {
        let doc_opt = verify_and_parse_attestation(vector::empty<u8>());
        assert!(doc_opt.is_none(), 0);

        let doc_opt_with_roots = verify_and_parse_attestation_with_roots(
            vector::empty<u8>(),
            vector[b"fake-root"],
            0,
        );
        assert!(doc_opt_with_roots.is_none(), 1);
        assert!(
            !verify_attestation_matches(
                vector::empty<u8>(),
                0,
                &b"pcr0",
                &b"nonce",
                &b"user-data",
                &b"public-key",
            ),
            2,
        );
        assert!(
            !verify_attestation_matches_with_roots(
                vector::empty<u8>(),
                vector[b"fake-root"],
                0,
                0,
                &b"pcr0",
                &b"nonce",
                &b"user-data",
                &b"public-key",
            ),
            3,
        );
        assert!(
            !verify_attestation_user_data(vector::empty<u8>(), &b"user-data"),
            4,
        );
        assert!(
            !verify_attestation_user_data_with_roots(
                vector::empty<u8>(),
                vector[b"fake-root"],
                0,
                &b"user-data",
            ),
            5,
        );
    }

    #[test]
    fun test_parsed_doc_accessors() {
        let doc = AttestationDoc {
            module_id: b"module-1",
            timestamp: 42,
            digest: b"SHA384",
            pcrs: vector[
                PcrEntry { index: 0, value: b"pcr0" },
                PcrEntry { index: 8, value: b"pcr8" },
            ],
            certificate: b"cert",
            user_data: option::some(b"user-data"),
            nonce: option::some(b"nonce"),
            public_key: option::some(b"public-key"),
        };

        assert!(module_id(&doc) == b"module-1", 0);
        assert!(timestamp(&doc) == 42, 1);
        assert!(digest(&doc) == b"SHA384", 2);
        assert!(certificate(&doc) == b"cert", 3);
        assert!(pcr_count(&doc) == 2, 4);

        let (pcr_index, pcr_value) = pcr_at(&doc, 1);
        assert!(pcr_index == 8, 5);
        assert!(pcr_value == b"pcr8", 6);

        let pcr0 = pcr(&doc, 0);
        assert!(pcr0.is_some(), 7);
        assert!(pcr0.borrow() == &b"pcr0", 8);
        assert!(pcr(&doc, 1).is_none(), 9);
        assert!(pcr_equals(&doc, 8, &b"pcr8"), 10);
        assert!(!pcr_equals(&doc, 8, &b"wrong"), 11);

        assert!(user_data_equals(&doc, &b"user-data"), 12);
        assert!(!user_data_equals(&doc, &b"wrong"), 13);
        assert!(nonce_equals(&doc, &b"nonce"), 14);
        assert!(public_key_equals(&doc, &b"public-key"), 15);
        assert!(user_data(&doc).borrow() == &b"user-data", 16);
        assert!(nonce(&doc).borrow() == &b"nonce", 17);
        assert!(public_key(&doc).borrow() == &b"public-key", 18);
    }

    #[test(aptos_framework = @0x1)]
    fun test_trusted_roots_store(aptos_framework: &signer) {
        initialize(aptos_framework, vector[b"root-a"]);
        assert!(is_initialized(), 0);
        assert!(trusted_root_count() == 1, 1);
        assert!(trusted_roots() == vector[b"root-a"], 2);
        assert!(contains_trusted_root(&b"root-a"), 3);
        assert!(!contains_trusted_root(&b"root-b"), 4);

        add_trusted_root(aptos_framework, b"root-b");
        assert!(trusted_root_count() == 2, 5);
        assert!(contains_trusted_root(&b"root-b"), 6);

        remove_trusted_root(aptos_framework, b"root-a");
        assert!(trusted_roots() == vector[b"root-b"], 7);

        set_trusted_roots(aptos_framework, vector[b"root-c"]);
        assert!(trusted_roots() == vector[b"root-c"], 8);

        assert!(!verify_attestation_from_store(vector::empty<u8>(), 0), 9);
        assert!(verify_and_parse_attestation_from_store(vector::empty<u8>(), 0).is_none(), 10);
        assert!(
            !verify_attestation_matches_from_store(
                vector::empty<u8>(),
                0,
                0,
                &b"pcr0",
                &b"nonce",
                &b"user-data",
                &b"public-key",
            ),
            11,
        );
    }
}
