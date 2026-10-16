/// A one-use challenge and verification state machine for Google Confidential Space.
/// The signing keys are the effective, quorum-observed trust value; the token and
/// the caller's expected program identity are never treated as trust anchors.
module aptos_framework::google_space {
    use std::error;
    use std::hash;
    use std::option;
    use std::option::Option;
    use aptos_framework::event::emit;
    use aptos_framework::randomness;
    use aptos_framework::timestamp;
    use aptos_framework::trust_observation;

    // Native aborts with this code for a malformed or invalid token or JWK set.
    const EINVALID_GOOGLE_PROOF: u64 = 1;
    const EREQUEST_PENDING: u64 = 2;
    const ENO_PENDING_REQUEST: u64 = 3;
    const EEXPIRED_REQUEST: u64 = 4;
    const EINVALID_REQUEST: u64 = 5;
    const ETRUST_ROOT_UNAVAILABLE: u64 = 6;
    const EWRONG_PROGRAM: u64 = 7;

    const GOOGLE_SPACE_JWKS_TARGET: vector<u8> = b"google_space_jwks";
    const STRICT_LAUNCH_CLAIMS: vector<u8> = b"{\"args\":[],\"cmd_override\":[],\"env\":{},\"env_override\":{}}";
    const MAX_AUDIENCE_BYTES: u64 = 512;
    const MAX_IMAGE_DIGEST_BYTES: u64 = 128;
    const KEY_COMMITMENT_BYTES: u64 = 32;
    const MAX_TOKEN_BYTES: u64 = 32 * 1024;
    const MAX_LAUNCH_CLAIMS_BYTES: u64 = 16 * 1024;
    const MAX_REQUEST_LIFETIME_SECS: u64 = 3600;
    const MAX_RESULT_AGE_SECS: u64 = 3600;

    /// This request is owned by the account that will consume the result.
    struct PendingRequest has copy, drop, store {
        request_id: u64,
        challenge: vector<u8>,
        key_commitment: vector<u8>,
        audience: vector<u8>,
        expected_image_digest: vector<u8>,
        expected_launch_claims: vector<u8>,
        started_at_secs: u64,
        expires_at_secs: u64,
    }

    /// Fields from the signed token, populated only by the Rust native after
    /// signature, issuer, challenge, audience, time, and platform checks pass.
    struct VerifiedGoogleSpaceClaims has copy, drop, store {
        image_digest: vector<u8>,
        image_reference: vector<u8>,
        project_id: vector<u8>,
        instance_id: vector<u8>,
        subject: vector<u8>,
        audience: vector<u8>,
        nonce: vector<u8>,
        hwmodel: vector<u8>,
        issued_at: u64,
        not_before: u64,
        expires_at: u64,
        signed_payload: vector<u8>,
        launch_claims: vector<u8>,
    }

    struct State has key {
        next_request_id: u64,
        pending: Option<PendingRequest>,
        last_verified: Option<VerifiedGoogleSpaceResult>,
    }

    /// The signed program claims and the worker key commitment associated with
    /// the signed challenge. The Google token does not itself prove possession
    /// of that key; applications must obtain the token from a workload that
    /// derives and controls the key, rather than blindly relaying a nonce.
    struct VerifiedGoogleSpaceResult has copy, drop, store {
        request_id: u64,
        key_commitment: vector<u8>,
        verified_at_secs: u64,
        jwks_sha3_256: vector<u8>,
        claims: VerifiedGoogleSpaceClaims,
    }

    #[event]
    struct ChallengeIssued has drop, store {
        owner: address,
        request_id: u64,
        /// Lowercase hex, exactly as supplied to the Google attestation API.
        nonce: vector<u8>,
        expires_at_secs: u64,
    }

    #[event]
    struct GoogleSpaceVerified has drop, store {
        owner: address,
        request_id: u64,
        claims: VerifiedGoogleSpaceClaims,
        key_commitment: vector<u8>,
    }

    /// Records a one-use request with an unpredictable 32-byte challenge from
    /// the private `#[randomness]` entry point. This function is never public.
    /// `expected_image_digest` is the exact `submods.container.image_digest`
    /// that the calling application accepts. `key_commitment` is a 32-byte
    /// digest of the worker key the application plans to register.
    fun begin_with_challenge(
        owner: &signer,
        audience: vector<u8>,
        expected_image_digest: vector<u8>,
        key_commitment: vector<u8>,
        lifetime_secs: u64,
        expected_launch_claims: vector<u8>,
        challenge: vector<u8>,
    ): vector<u8> {
        assert!(audience.length() > 0 && audience.length() <= MAX_AUDIENCE_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(expected_image_digest.length() > 0
            && expected_image_digest.length() <= MAX_IMAGE_DIGEST_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(key_commitment.length() == KEY_COMMITMENT_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(challenge.length() == KEY_COMMITMENT_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(expected_launch_claims.length() > 0
            && expected_launch_claims.length() <= MAX_LAUNCH_CLAIMS_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(lifetime_secs > 0 && lifetime_secs <= MAX_REQUEST_LIFETIME_SECS,
            error::invalid_argument(EINVALID_REQUEST));
        let addr = owner.address_of();
        let now = timestamp::now_seconds();
        if (!exists<State>(addr)) {
            move_to(owner, State {
                next_request_id: 0,
                pending: option::none(),
                last_verified: option::none(),
            });
        };
        let state = &mut State[addr];
        if (state.pending.is_some()) {
            assert!(state.pending.borrow().expires_at_secs <= now,
                error::invalid_state(EREQUEST_PENDING));
            state.pending.extract();
        };
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        let nonce = hex_encode(&challenge);
        let expires_at_secs = now + lifetime_secs;
        state.pending.fill(PendingRequest {
            request_id,
            challenge,
            key_commitment,
            audience,
            expected_image_digest,
            expected_launch_claims,
            started_at_secs: now,
            expires_at_secs,
        });
        emit(ChallengeIssued { owner: addr, request_id, nonce: copy nonce, expires_at_secs });
        nonce
    }

    #[randomness]
    /// Transaction entry point; the nonce is emitted and available through
    /// `pending_nonce` after this transaction commits.
    entry fun begin_entry(
        owner: &signer,
        audience: vector<u8>,
        expected_image_digest: vector<u8>,
        key_commitment: vector<u8>,
        lifetime_secs: u64,
    ) {
        begin_with_challenge(owner, audience, expected_image_digest, key_commitment,
            lifetime_secs, STRICT_LAUNCH_CLAIMS, randomness::bytes(32));
    }

    #[randomness]
    /// Starts a request that approves the exact signed `args`, `cmd_override`,
    /// `env`, and `env_override` claims. The JSON object must have those four
    /// fields with array/object types. All environment keys and values must
    /// match exactly; unspecified additions are rejected.
    entry fun begin_entry_with_launch_claims(
        owner: &signer,
        audience: vector<u8>,
        expected_image_digest: vector<u8>,
        key_commitment: vector<u8>,
        lifetime_secs: u64,
        expected_launch_claims: vector<u8>,
    ) {
        begin_with_challenge(owner, audience, expected_image_digest, key_commitment,
            lifetime_secs, expected_launch_claims, randomness::bytes(32));
    }

    /// Cancels a pending request. The request counter still advances, so its
    /// challenge is never reused.
    public entry fun cancel(owner: &signer) {
        let addr = owner.address_of();
        assert!(exists<State>(addr) && State[addr].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        State[addr].pending.extract();
    }

    /// Verifies the token against the current effective JWK set and commits a
    /// result only if the signed program digest matches the pending policy.
    public fun verify(owner: &signer, token: vector<u8>): VerifiedGoogleSpaceResult {
        let addr = owner.address_of();
        assert!(exists<State>(addr) && State[addr].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        assert!(token.length() > 0 && token.length() <= MAX_TOKEN_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        let pending = *State[addr].pending.borrow();
        let now = timestamp::now_seconds();
        assert!(now < pending.expires_at_secs, error::invalid_state(EEXPIRED_REQUEST));
        let jwks = trust_observation::get_effective_value(GOOGLE_SPACE_JWKS_TARGET);
        assert!(jwks.is_some(), error::invalid_state(ETRUST_ROOT_UNAVAILABLE));
        let nonce = hex_encode(&pending.challenge);
        let jwks_value = jwks.extract();
        let jwks_sha3_256 = hash::sha3_256(copy jwks_value);
        let claims = verify_google_space_token(
            token,
            jwks_value,
            pending.audience,
            copy nonce,
            pending.expected_launch_claims,
            now,
        );
        assert!(claims.issued_at >= pending.started_at_secs, EINVALID_GOOGLE_PROOF);
        assert!(claims.image_digest == pending.expected_image_digest,
            error::permission_denied(EWRONG_PROGRAM));
        assert!(claims.audience == pending.audience && claims.nonce == nonce,
            error::invalid_state(EINVALID_GOOGLE_PROOF));
        let state = &mut State[addr];
        state.pending.extract();
        let result = VerifiedGoogleSpaceResult {
            request_id: pending.request_id,
            key_commitment: pending.key_commitment,
            verified_at_secs: now,
            jwks_sha3_256,
            claims: copy claims,
        };
        state.last_verified = option::some(copy result);
        emit(GoogleSpaceVerified {
            owner: addr,
            request_id: pending.request_id,
            claims,
            key_commitment: pending.key_commitment,
        });
        result
    }

    /// Transaction entry point; verified claims are emitted and saved for a
    /// short-lived read through `last_verified` and `is_current`.
    public entry fun verify_entry(owner: &signer, token: vector<u8>) {
        verify(owner, token);
    }

    #[view]
    public fun pending_nonce(owner: address): Option<vector<u8>> {
        if (!exists<State>(owner) || State[owner].pending.is_none()) {
            option::none()
        } else {
            option::some(hex_encode(&State[owner].pending.borrow().challenge))
        }
    }

    #[view]
    public fun last_verified(owner: address): Option<VerifiedGoogleSpaceResult> {
        if (!exists<State>(owner)) { option::none() } else { State[owner].last_verified }
    }

    #[view]
    /// Checks the current chain clock and effective JWK set before a stored
    /// result is used. The caller chooses a stricter age if needed.
    public fun is_current(owner: address, max_age_secs: u64): bool {
        if (max_age_secs > MAX_RESULT_AGE_SECS || !exists<State>(owner)
            || State[owner].last_verified.is_none()) { return false };
        let result = State[owner].last_verified.borrow();
        let now = timestamp::now_seconds();
        if (now < result.verified_at_secs || now >= result.claims.expires_at
            || now - result.verified_at_secs > max_age_secs) { return false };
        let jwks = trust_observation::get_effective_value(GOOGLE_SPACE_JWKS_TARGET);
        jwks.is_some() && hash::sha3_256(jwks.extract()) == result.jwks_sha3_256
    }

    public fun request_id(self: &VerifiedGoogleSpaceResult): u64 { self.request_id }

    public fun key_commitment(self: &VerifiedGoogleSpaceResult): &vector<u8> { &self.key_commitment }

    public fun verified_at_secs(self: &VerifiedGoogleSpaceResult): u64 { self.verified_at_secs }

    public fun jwks_sha3_256(self: &VerifiedGoogleSpaceResult): &vector<u8> { &self.jwks_sha3_256 }

    public fun claims(self: &VerifiedGoogleSpaceResult): &VerifiedGoogleSpaceClaims { &self.claims }

    public fun image_digest(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.image_digest }

    public fun image_reference(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.image_reference }

    public fun project_id(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.project_id }

    public fun instance_id(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.instance_id }

    public fun subject(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.subject }

    public fun audience(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.audience }

    public fun nonce(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.nonce }

    public fun hwmodel(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.hwmodel }

    public fun issued_at(self: &VerifiedGoogleSpaceClaims): u64 { self.issued_at }

    public fun not_before(self: &VerifiedGoogleSpaceClaims): u64 { self.not_before }

    public fun expires_at(self: &VerifiedGoogleSpaceClaims): u64 { self.expires_at }

    public fun signed_payload(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.signed_payload }

    public fun launch_claims(self: &VerifiedGoogleSpaceClaims): &vector<u8> { &self.launch_claims }

    fun hex_encode(bytes: &vector<u8>): vector<u8> {
        let digits = b"0123456789abcdef";
        let result = vector[];
        let i = 0;
        while (i < bytes.length()) {
            let b = bytes[i];
            result.push_back(digits[(b / 16) as u64]);
            result.push_back(digits[(b % 16) as u64]);
            i += 1;
        };
        result
    }

    native fun verify_google_space_token(
        token: vector<u8>,
        trusted_jwks: vector<u8>,
        expected_audience: vector<u8>,
        expected_nonce: vector<u8>,
        expected_launch_claims: vector<u8>,
        now_secs: u64,
    ): VerifiedGoogleSpaceClaims;

    #[test_only]
    const TEST_JWKS: vector<u8> = b"{\"keys\":[{\"kty\":\"RSA\",\"n\":\"6S7asUuzq5Q_3U9rbs-PkDVIdjgmtgWreG5qWPsC9xXZKiMV1AiV9LXyqQsAYpCqEDM3XbfmZqGb48yLhb_XqZaKgSYaC_h2DjM7lgrIQAp9902Rr8fUmLN2ivr5tnLxUUOnMOc2SQtr9dgzTONYW5Zu3PwyvAWk5D6ueIUhLtYzpcB-etoNdL3Ir2746KIy_VUsDwAM7dhrqSK8U2xFCGlau4ikOTtvzDownAMHMrfE7q1B6WZQDAQlBmxRQsyKln5DIsKv6xauNsHRgBAKctUxZG8M4QJIx3S6Aughd3RZC4Ca5Ae9fd8L8mlNYBCrQhOZ7dS0f4at4arlLcajtw\",\"e\":\"AQAB\",\"kid\":\"test-rsa\",\"alg\":\"RS256\"}]}";
    #[test_only]
    const TEST_TOKEN: vector<u8> = b"eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3QtcnNhIiwidHlwIjoiSldUIn0.eyJpc3MiOiJodHRwczovL2NvbmZpZGVudGlhbGNvbXB1dGluZy5nb29nbGVhcGlzLmNvbSIsImF1ZCI6ImFwdG9zLWF0dGVzdGF0aW9uLXRlc3QiLCJzdWIiOiJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9jb21wdXRlL3YxL3Byb2plY3RzL2V4YW1wbGUvem9uZXMvdXMtY2VudHJhbDEtYS9pbnN0YW5jZXMvMTIzIiwiZXhwIjoxNzAwMDAzNjAwLCJpYXQiOjE2OTk5OTk5OTAsIm5iZiI6MTY5OTk5OTk5MCwiZWF0X25vbmNlIjpbIjUwZTg1ZDk0MjA3NzZmZDZhNTEyNmFhZmY3YWZkM2E4MzBmMTgwMmIwZDFlODk3ZDM1MDA0NWI0YzAwNTJiNTkiXSwib2VtaWQiOjExMTI5LCJzZWNib290Ijp0cnVlLCJod21vZGVsIjoiR0NQX0lOVEVMX1REWCIsInN3bmFtZSI6IkNPTkZJREVOVElBTF9TUEFDRSIsImRiZ3N0YXQiOiJkaXNhYmxlZC1zaW5jZS1ib290Iiwic3VibW9kcyI6eyJjb250YWluZXIiOnsiaW1hZ2VfZGlnZXN0Ijoic2hhMjU2OmFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWEiLCJpbWFnZV9yZWZlcmVuY2UiOiJleGFtcGxlLmNvbS93b3JrbG9hZDpsYXRlc3QifSwiZ2NlIjp7InByb2plY3RfaWQiOiJleGFtcGxlIiwiaW5zdGFuY2VfaWQiOiIxMjMifX19.itvurNHfwhj90VC3GjIxRx19i-BkNJsGtdNQIOb7IFEHdNuOV7juKdOvF7805w2MHrTb8ezjV0ptnU6uJeVAP-tm0vJ29UIUFHs_4GBXFM1CNLECnNhHGIfxCgY9WG14WfzkQ1lEnG9gMC3pECb_2-ysi6KxG-HUdXBd2p-F_m8OX67hdExNrgI7gwCBWx8-N79LpNpmyG2O3KfVbAfX2YI7ftSRonPuJ4RUGHU6zYDZXmDBYtPlxAgz8-WKBnTjKl5USiZB0CKQV21KguOlve7q4CgTO3bvoXEUrXbe35jKmY7sqJ916vfeBLXsqV4_4PmxN4jkeDNhUwmDc-sEzQ";
    #[test_only]
    const TEST_NONCE: vector<u8> = b"50e85d9420776fd6a5126aaff7afd3a830f1802b0d1e897d350045b4c0052b59";
    #[test_only]
    const TEST_CHALLENGE: vector<u8> = x"50e85d9420776fd6a5126aaff7afd3a830f1802b0d1e897d350045b4c0052b59";
    #[test_only]
    const TEST_IMAGE_DIGEST: vector<u8> = b"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test_only]
    fun setup_test(framework: &signer) {
        aptos_framework::account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(framework);
        timestamp::update_global_time_for_test_secs(1699999990);
        aptos_framework::config_buffer::initialize(framework);
        aptos_framework::reconfiguration::initialize_for_test(framework);
        trust_observation::initialize(framework);
        trust_observation::upsert_target_for_next_epoch(
            framework, GOOGLE_SPACE_JWKS_TARGET,
            std::string::utf8(b"https://www.googleapis.com/service_accounts/v1/metadata/jwk/signer@confidentialspace-sign.iam.gserviceaccount.com"),
            2, 4096,
        );
        trust_observation::activate_for_test(framework);
        trust_observation::set_replacement_patch(framework, GOOGLE_SPACE_JWKS_TARGET, TEST_JWKS);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_verified_token_and_root_revocation(framework: &signer, owner: &signer) {
        setup_test(framework);
        let key_commitment = vector[];
        for (_i in 0..32) { key_commitment.push_back(1); };
        let challenge = begin_with_challenge(owner, b"aptos-attestation-test", TEST_IMAGE_DIGEST,
            copy key_commitment, 60, STRICT_LAUNCH_CLAIMS, TEST_CHALLENGE);
        assert!(challenge == TEST_NONCE, 1);
        let result = verify(owner, TEST_TOKEN);
        assert!(*result.key_commitment() == key_commitment, 2);
        let claims = result.claims();
        assert!(*claims.image_digest() == TEST_IMAGE_DIGEST, 3);
        assert!(*claims.project_id() == b"example", 4);
        assert!(*claims.instance_id() == b"123", 5);
        assert!(*claims.nonce() == TEST_NONCE, 6);
        assert!(*claims.launch_claims() == STRICT_LAUNCH_CLAIMS, 10);
        assert!(pending_nonce(owner.address_of()).is_none(), 7);
        assert!(is_current(owner.address_of(), 60), 8);
        trust_observation::set_removal_patch(framework, GOOGLE_SPACE_JWKS_TARGET);
        assert!(!is_current(owner.address_of(), 60), 9);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = EINVALID_GOOGLE_PROOF, location = Self)]
    fun test_reject_wrong_nonce(framework: &signer, owner: &signer) {
        setup_test(framework);
        let other_key = vector[];
        for (_i in 0..32) { other_key.push_back(2); };
        begin_with_challenge(owner, b"aptos-attestation-test", TEST_IMAGE_DIGEST,
            other_key, 60, STRICT_LAUNCH_CLAIMS, x"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        verify(owner, TEST_TOKEN);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = EINVALID_GOOGLE_PROOF, location = Self)]
    fun test_reject_bad_signature(framework: &signer, owner: &signer) {
        setup_test(framework);
        let key_commitment = vector[];
        for (_i in 0..32) { key_commitment.push_back(1); };
        begin_with_challenge(owner, b"aptos-attestation-test", TEST_IMAGE_DIGEST,
            key_commitment, 60, STRICT_LAUNCH_CLAIMS, TEST_CHALLENGE);
        let token = TEST_TOKEN;
        let old = token[1100];
        token[1100] = if (old == 65) { 66 } else { 65 };
        verify(owner, token);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_challenge_lifecycle(framework: &signer, owner: &signer) {
        timestamp::set_time_has_started_for_testing(framework);
        let key_commitment = vector[];
        for (i in 0..32) { key_commitment.push_back(1); };
        let first = begin_with_challenge(owner, b"audience", b"sha256:digest",
            copy key_commitment, 60, STRICT_LAUNCH_CLAIMS, x"0101010101010101010101010101010101010101010101010101010101010101");
        assert!(first.length() == 64, 1);
        assert!(pending_nonce(owner.address_of()).extract() == first, 2);
        cancel(owner);
        assert!(pending_nonce(owner.address_of()).is_none(), 3);
        let second = begin_with_challenge(owner, b"audience", b"sha256:digest",
            key_commitment, 60, STRICT_LAUNCH_CLAIMS, x"0202020202020202020202020202020202020202020202020202020202020202");
        assert!(second != first, 4);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = EINVALID_GOOGLE_PROOF, location = Self)]
    fun test_reject_token_issued_before_request(framework: &signer, owner: &signer) {
        setup_test(framework);
        timestamp::update_global_time_for_test_secs(1700000000);
        let key_commitment = vector[];
        for (_i in 0..32) { key_commitment.push_back(1); };
        begin_with_challenge(owner, b"aptos-attestation-test", TEST_IMAGE_DIGEST,
            key_commitment, 60, STRICT_LAUNCH_CLAIMS, TEST_CHALLENGE);
        verify(owner, TEST_TOKEN);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_is_current_rejects_exact_expiry(framework: &signer, owner: &signer) {
        setup_test(framework);
        let key_commitment = vector[];
        for (_i in 0..32) { key_commitment.push_back(1); };
        begin_with_challenge(owner, b"aptos-attestation-test", TEST_IMAGE_DIGEST,
            key_commitment, 60, STRICT_LAUNCH_CLAIMS, TEST_CHALLENGE);
        verify(owner, TEST_TOKEN);
        timestamp::update_global_time_for_test_secs(1700003600);
        assert!(!is_current(owner.address_of(), 3600), 1);
    }
}
