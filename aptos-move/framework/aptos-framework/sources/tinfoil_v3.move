/// A one-use, multi-transaction verifier for Tinfoil v3 envelopes carrying
/// legacy FormatV2 (`codeReferences`) collateral. The newer FormatV3
/// (`configReferences`) protocol needs separate verifier stages and is rejected
/// by the native parser until those stages are implemented.
/// Each uploaded fragment is untrusted evidence. Intel and Sigstore roots come only
/// from effective quorum-observed chain state, and repositories are approved
/// by Aptos framework governance.
module aptos_framework::tinfoil_v3 {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::option;
    use std::option::Option;
    use aptos_std::table_with_length;
    use aptos_std::table_with_length::TableWithLength;
    use aptos_framework::chain_id;
    use aptos_framework::event::emit;
    use aptos_framework::randomness;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::trust_observation;

    // Native aborts with this code when supported FormatV2 collateral fails.
    const EINVALID_TINFOIL_PROOF: u64 = 1;
    const EINVALID_REPOSITORY: u64 = 2;
    const EUNAPPROVED_REPOSITORY: u64 = 3;
    const ETOO_MANY_REPOSITORIES: u64 = 4;
    const EREQUEST_PENDING: u64 = 5;
    const ENO_PENDING_REQUEST: u64 = 6;
    const EEXPIRED_REQUEST: u64 = 7;
    const EINVALID_REQUEST: u64 = 8;
    const ETRUST_ROOT_UNAVAILABLE: u64 = 9;
    const EINVALID_VERIFIED_CLAIMS: u64 = 10;
    const ENOT_READY_FOR_CLEANUP: u64 = 11;
    const EWRONG_STAGE: u64 = 12;
    const ETRUST_ROOT_CHANGED: u64 = 13;

    const INTEL_ROOT_TARGET: vector<u8> = b"intel_sgx_root_ca";
    const SIGSTORE_ROOT_TARGET: vector<u8> = b"sigstore_trusted_root";
    const CHALLENGE_DOMAIN: vector<u8> = b"aptos.tinfoil_v3.v1";
    const MAX_REPOSITORIES: u64 = 16;
    const MAX_REPOSITORY_BYTES: u64 = 200;
    const TLS_SPKI_SHA256_BYTES: u64 = 32;
    const MAX_CHUNK_BYTES: u64 = 48 * 1024;
    const MAX_DOCUMENT_BYTES: u64 = 4 * 1024 * 1024;
    const MAX_STAGE_BYTES: u64 = 1024 * 1024;
    const MAX_CHUNKS: u64 = 96;
    const MAX_CLEANUP_CHUNKS_PER_CALL: u64 = 8;
    const MAX_REQUEST_LIFETIME_SECS: u64 = 3600;
    const MAX_RESULT_AGE_SECS: u64 = 3600;
    const STAGE_QUOTE: u8 = 0;
    const STAGE_CODE: u8 = 1;
    const STAGE_CODE_FRESHNESS: u8 = 2;
    const STAGE_PLATFORM: u8 = 3;
    const STAGE_PLATFORM_FRESHNESS: u8 = 4;
    const STAGE_COMPLETE: u8 = 5;

    struct ChallengeMaterial has drop {
        domain: vector<u8>,
        chain_id: u8,
        owner: address,
        request_id: u64,
        entropy: vector<u8>,
    }

    /// The framework allowlist is empty until governance explicitly adds a
    /// repository. A revocation takes effect even for an in-flight request.
    struct ApprovedRepositories has key {
        entries: vector<vector<u8>>,
    }

    struct PendingRequest has store {
        request_id: u64,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        expected_tls_spki_sha256: vector<u8>,
        expires_at_secs: u64,
        chunks: TableWithLength<u64, vector<u8>>,
        chunk_count: u64,
        total_bytes: u64,
        stage: u8,
        stage_start_chunk: u64,
        stage_bytes: u64,
        proof_time_secs: u64,
        intel_root_sha3_256: vector<u8>,
        sigstore_root_sha3_256: vector<u8>,
        quote: Option<QuoteStage>,
        code: Option<CodeStage>,
        platform: Option<PlatformStage>,
        /// Verification or cancellation closes the request before cleanup.
        closed: bool,
        cleanup_cursor: u64,
    }

    /// Authenticated TDX quote output. The native checks REPORTDATA, DCAP,
    /// collateral signatures and the Intel root before constructing this.
    struct QuoteStage has copy, drop, store {
        report_data: vector<u8>,
        tls_spki_sha256: vector<u8>,
        hpke_key: vector<u8>,
        quote_sha256: vector<u8>,
        mrtd: vector<u8>,
        rtmr0: vector<u8>,
        rtmr1: vector<u8>,
        rtmr2: vector<u8>,
        rtmr3: vector<u8>,
        tcb_status: vector<u8>,
        ppid: vector<u8>,
        qe_vendor_id: vector<u8>,
        tee_tcb_svn: vector<u8>,
        mr_seam: vector<u8>,
        td_attributes: vector<u8>,
        xfam: vector<u8>,
        tcb_evaluation_data_number: u64,
    }

    /// Authenticated code provenance after Sigstore verification.
    struct CodeStage has copy, drop, store {
        repo: vector<u8>,
        tag: vector<u8>,
        commit: vector<u8>,
        subject_name: vector<u8>,
        digest: vector<u8>,
        shape_cpus: u64,
        shape_memory_mb: u64,
        shape_gpus: u64,
        shape_disks: u64,
    }

    /// Authenticated platform endorsement after Sigstore verification.
    struct PlatformStage has copy, drop, store {
        repo: vector<u8>,
        tag: vector<u8>,
        commit: vector<u8>,
        subject_name: vector<u8>,
        digest: vector<u8>,
        measurement_id: vector<u8>,
    }

    /// Fields returned only after Rust validates all supported FormatV2
    /// collateral: TDX quote, Intel evidence, Sigstore provenance, freshness,
    /// and the report-data join.
    struct VerifiedTinfoilV3Claims has copy, drop, store {
        tls_spki_sha256: vector<u8>,
        hpke_key: vector<u8>,
        release_digest: vector<u8>,
        trusted_repo: vector<u8>,
        nonce: vector<u8>,
        mrtd: vector<u8>,
        rtmr0: vector<u8>,
        rtmr1: vector<u8>,
        rtmr2: vector<u8>,
        rtmr3: vector<u8>,
        tcb_status: vector<u8>,
        release_tag: vector<u8>,
        release_commit: vector<u8>,
        platform_measurement_id: vector<u8>,
    }

    struct VerifiedTinfoilV3Result has copy, drop, store {
        request_id: u64,
        verified_at_secs: u64,
        intel_root_sha3_256: vector<u8>,
        sigstore_root_sha3_256: vector<u8>,
        claims: VerifiedTinfoilV3Claims,
    }

    struct State has key {
        next_request_id: u64,
        pending: Option<PendingRequest>,
        last_verified: Option<VerifiedTinfoilV3Result>,
    }

    #[event]
    struct ChallengeIssued has drop, store {
        owner: address,
        request_id: u64,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        expires_at_secs: u64,
    }

    #[event]
    struct TinfoilV3FormatV2Verified has drop, store {
        owner: address,
        result: VerifiedTinfoilV3Result,
    }

    #[event]
    struct StageVerified has drop, store {
        owner: address,
        request_id: u64,
        completed_stage: u8,
    }

    /// Governance explicitly enables or revokes a GitHub repository. This
    /// string must match the repository identity in verified Sigstore bundles.
    public fun set_repository_approved(framework: &signer, repo: vector<u8>, approved: bool) {
        system_addresses::assert_aptos_framework(framework);
        assert!(repo.length() > 0 && repo.length() <= MAX_REPOSITORY_BYTES,
            error::invalid_argument(EINVALID_REPOSITORY));
        if (!exists<ApprovedRepositories>(@aptos_framework)) {
            move_to(framework, ApprovedRepositories { entries: vector[] });
        };
        let entries = &mut ApprovedRepositories[@aptos_framework].entries;
        let (found, index) = entries.index_of(&repo);
        if (approved && !found) {
            assert!(entries.length() < MAX_REPOSITORIES,
                error::resource_exhausted(ETOO_MANY_REPOSITORIES));
            entries.push_back(repo);
        } else if (!approved && found) {
            entries.remove(index);
        };
    }

    #[view]
    public fun is_repository_approved(repo: vector<u8>): bool {
        exists<ApprovedRepositories>(@aptos_framework)
            && ApprovedRepositories[@aptos_framework].entries.contains(&repo)
    }

    /// Creates a fresh nonce from validator randomness, scoped to this chain,
    /// account and request ID. The v3 document must quote exactly these 32 bytes.
    /// This must only be called by the private `#[randomness]` entry point.
    fun begin(
        owner: &signer,
        trusted_repo: vector<u8>,
        expected_tls_spki_sha256: vector<u8>,
        lifetime_secs: u64,
    ): vector<u8> {
        assert!(is_repository_approved(copy trusted_repo),
            error::permission_denied(EUNAPPROVED_REPOSITORY));
        assert!(expected_tls_spki_sha256.length() == TLS_SPKI_SHA256_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        assert!(lifetime_secs > 0 && lifetime_secs <= MAX_REQUEST_LIFETIME_SECS,
            error::invalid_argument(EINVALID_REQUEST));
        let addr = owner.address_of();
        if (!exists<State>(addr)) {
            move_to(owner, State {
                next_request_id: 0,
                pending: option::none(),
                last_verified: option::none(),
            });
        };
        let state = &mut State[addr];
        assert!(state.pending.is_none(), error::invalid_state(EREQUEST_PENDING));
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        let nonce = hash::sha3_256(bcs::to_bytes(&ChallengeMaterial {
            domain: CHALLENGE_DOMAIN,
            chain_id: chain_id::get(),
            owner: addr,
            request_id,
            entropy: randomness::bytes(32),
        }));
        let expires_at_secs = timestamp::now_seconds() + lifetime_secs;
        state.pending.fill(PendingRequest {
            request_id,
            nonce: copy nonce,
            trusted_repo: copy trusted_repo,
            expected_tls_spki_sha256,
            expires_at_secs,
            chunks: table_with_length::new(),
            chunk_count: 0,
            total_bytes: 0,
            stage: STAGE_QUOTE,
            stage_start_chunk: 0,
            stage_bytes: 0,
            proof_time_secs: 0,
            intel_root_sha3_256: vector[],
            sigstore_root_sha3_256: vector[],
            quote: option::none(),
            code: option::none(),
            platform: option::none(),
            closed: false,
            cleanup_cursor: 0,
        });
        emit(ChallengeIssued { owner: addr, request_id, nonce: copy nonce, trusted_repo, expires_at_secs });
        nonce
    }

    #[randomness]
    /// Transaction entry point; reads the nonce from `pending_nonce` or the event.
    /// It is private so a caller cannot inspect and abort on the random result.
    entry fun begin_entry(
        owner: &signer,
        trusted_repo: vector<u8>,
        expected_tls_spki_sha256: vector<u8>,
        lifetime_secs: u64,
    ) {
        begin(owner, trusted_repo, expected_tls_spki_sha256, lifetime_secs);
    }

    /// Uploads one stage's fragment in transaction-sized chunks. A stage
    /// transition seals its current fragment; subsequent chunks belong only
    /// to the next stage.
    public entry fun append_chunk(owner: &signer, chunk: vector<u8>) {
        let addr = owner.address_of();
        assert!(exists<State>(addr) && State[addr].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        assert!(chunk.length() > 0 && chunk.length() <= MAX_CHUNK_BYTES,
            error::invalid_argument(EINVALID_REQUEST));
        let pending = State[addr].pending.borrow_mut();
        assert!(!pending.closed, error::invalid_state(ENO_PENDING_REQUEST));
        assert!(pending.stage < STAGE_COMPLETE, error::invalid_state(EWRONG_STAGE));
        assert!(timestamp::now_seconds() <= pending.expires_at_secs,
            error::invalid_state(EEXPIRED_REQUEST));
        assert!(pending.chunk_count < MAX_CHUNKS
            && pending.total_bytes + chunk.length() <= MAX_DOCUMENT_BYTES
            && pending.stage_bytes + chunk.length() <= MAX_STAGE_BYTES,
            error::resource_exhausted(EINVALID_REQUEST));
        pending.total_bytes += chunk.length();
        pending.stage_bytes += chunk.length();
        pending.chunks.add(pending.chunk_count, chunk);
        pending.chunk_count += 1;
    }

    /// Closes a request. Its chunks can be removed in bounded transactions with
    /// `clean_up`, including by a third party after the request expires.
    public entry fun cancel(owner: &signer) {
        let addr = owner.address_of();
        assert!(exists<State>(addr) && State[addr].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        State[addr].pending.borrow_mut().closed = true;
        clean_up(addr, MAX_CLEANUP_CHUNKS_PER_CALL);
    }

    /// Frees at most eight staged chunks. Anyone can clean up an expired or
    /// completed request; no untrusted caller can erase an active one.
    public entry fun clean_up(owner: address, max_chunks: u64) {
        assert!(exists<State>(owner) && State[owner].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        assert!(max_chunks > 0 && max_chunks <= MAX_CLEANUP_CHUNKS_PER_CALL,
            error::invalid_argument(EINVALID_REQUEST));
        let pending = State[owner].pending.borrow_mut();
        assert!(pending.closed || timestamp::now_seconds() > pending.expires_at_secs,
            error::invalid_state(ENOT_READY_FOR_CLEANUP));
        let removed = 0;
        while (removed < max_chunks && pending.cleanup_cursor < pending.chunk_count) {
            pending.chunks.remove(pending.cleanup_cursor);
            pending.cleanup_cursor += 1;
            removed += 1;
        };
        if (pending.cleanup_cursor == pending.chunk_count) {
            let consumed = State[owner].pending.extract();
            destroy_empty_pending(consumed);
        };
    }

    /// Stage 0: verify the TDX quote and Intel collateral, then pin both
    /// effective roots and the chain time used for every later stage.
    public entry fun verify_quote(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_QUOTE);
        let pending = State[addr].pending.borrow();
        let (intel_root, sigstore_root) = effective_roots();
        let intel_root_hash = hash::sha3_256(copy intel_root);
        let sigstore_root_hash = hash::sha3_256(sigstore_root);
        let proof_time_secs = timestamp::now_seconds();
        let quote = verify_tinfoil_v3_quote(
            current_fragment(pending),
            pending.nonce,
            proof_time_secs,
            intel_root,
        );
        assert_quote_claims(pending, &quote);
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.quote.fill(quote);
        pending.proof_time_secs = proof_time_secs;
        pending.intel_root_sha3_256 = intel_root_hash;
        pending.sigstore_root_sha3_256 = sigstore_root_hash;
        advance_stage(pending, STAGE_CODE);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_QUOTE });
    }

    /// Stage 1: authenticate the code-release Sigstore bundle and compare its
    /// reference values with the already authenticated TDX measurements.
    public entry fun verify_code(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_CODE);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        let code = verify_tinfoil_v3_code(
            current_fragment(pending),
            pending.nonce,
            pending.trusted_repo,
            sigstore_root,
            *pending.quote.borrow(),
        );
        assert_code_claims(pending, &code);
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.code.fill(code);
        advance_stage(pending, STAGE_CODE_FRESHNESS);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_CODE });
    }

    /// Stage 2: authenticate the freshness witness for the code release.
    public entry fun verify_code_freshness(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_CODE_FRESHNESS);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        verify_tinfoil_v3_code_freshness(
            current_fragment(pending), pending.nonce, pending.proof_time_secs,
            sigstore_root, *pending.quote.borrow(), *pending.code.borrow(),
        );
        let request_id = pending.request_id;
        advance_stage(State[addr].pending.borrow_mut(), STAGE_PLATFORM);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_CODE_FRESHNESS });
    }

    /// Stage 3: authenticate the platform endorsement and match its reference
    /// measurements and platform identity to the TDX quote.
    public entry fun verify_platform(owner: &signer) {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_PLATFORM);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        let platform = verify_tinfoil_v3_platform(
            current_fragment(pending), pending.nonce, pending.proof_time_secs,
            sigstore_root, *pending.quote.borrow(), *pending.code.borrow(),
        );
        assert!(platform.digest.length() == 32 && platform.measurement_id.length() > 0,
            error::invalid_state(EINVALID_VERIFIED_CLAIMS));
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.platform.fill(platform);
        advance_stage(pending, STAGE_PLATFORM_FRESHNESS);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_PLATFORM });
    }

    /// Stage 4: authenticate the platform freshness witness. Only this stage
    /// can publish a final result, joining native-verified private outputs.
    public fun verify_platform_freshness(owner: &signer): VerifiedTinfoilV3Result {
        let addr = owner.address_of();
        assert_open_stage(addr, STAGE_PLATFORM_FRESHNESS);
        let pending = State[addr].pending.borrow();
        let (_, sigstore_root) = checked_roots(pending);
        verify_tinfoil_v3_platform_freshness(
            current_fragment(pending), pending.nonce, pending.proof_time_secs,
            sigstore_root, *pending.quote.borrow(), *pending.platform.borrow(),
        );
        let quote = *pending.quote.borrow();
        let code = *pending.code.borrow();
        let platform = *pending.platform.borrow();
        let claims = VerifiedTinfoilV3Claims {
            tls_spki_sha256: quote.tls_spki_sha256,
            hpke_key: quote.hpke_key,
            release_digest: code.digest,
            trusted_repo: pending.trusted_repo,
            nonce: pending.nonce,
            mrtd: quote.mrtd,
            rtmr0: quote.rtmr0,
            rtmr1: quote.rtmr1,
            rtmr2: quote.rtmr2,
            rtmr3: quote.rtmr3,
            tcb_status: quote.tcb_status,
            release_tag: code.tag,
            release_commit: code.commit,
            platform_measurement_id: platform.measurement_id,
        };
        let result = VerifiedTinfoilV3Result {
            request_id: pending.request_id,
            verified_at_secs: pending.proof_time_secs,
            intel_root_sha3_256: pending.intel_root_sha3_256,
            sigstore_root_sha3_256: pending.sigstore_root_sha3_256,
            claims,
        };
        let request_id = pending.request_id;
        let pending = State[addr].pending.borrow_mut();
        pending.closed = true;
        advance_stage(pending, STAGE_COMPLETE);
        State[addr].last_verified = option::some(copy result);
        emit(StageVerified { owner: addr, request_id, completed_stage: STAGE_PLATFORM_FRESHNESS });
        emit(TinfoilV3FormatV2Verified { owner: addr, result: copy result });
        result
    }

    /// Transaction entry point; a successful result is stored and emitted.
    public entry fun verify_entry(owner: &signer) {
        verify_platform_freshness(owner);
    }

    #[view]
    public fun pending_nonce(owner: address): Option<vector<u8>> {
        if (!exists<State>(owner) || State[owner].pending.is_none()
            || State[owner].pending.borrow().closed) {
            option::none()
        } else {
            option::some(State[owner].pending.borrow().nonce)
        }
    }

    // The next proof stage, or none when no active request exists.
    #[view]
    public fun pending_stage(owner: address): Option<u8> {
        if (!exists<State>(owner) || State[owner].pending.is_none()
            || State[owner].pending.borrow().closed) {
            option::none()
        } else {
            option::some(State[owner].pending.borrow().stage)
        }
    }

    // The result proves only v3-envelope / legacy FormatV2 collateral, and is
    // current only while both trust roots and the repository approval remain.
    #[view]
    public fun last_verified(owner: address): Option<VerifiedTinfoilV3Result> {
        if (!exists<State>(owner)) { option::none() } else { State[owner].last_verified }
    }

    #[view]
    /// Returns false after root rotation, repository revocation or age limit.
    /// Applications may impose a shorter `max_age_secs` than one hour.
    public fun is_current(owner: address, max_age_secs: u64): bool {
        if (max_age_secs > MAX_RESULT_AGE_SECS || !exists<State>(owner)
            || State[owner].last_verified.is_none()) { return false };
        let result = State[owner].last_verified.borrow();
        let now = timestamp::now_seconds();
        if (now < result.verified_at_secs || now - result.verified_at_secs > max_age_secs
            || !is_repository_approved(result.claims.trusted_repo)) { return false };
        let intel_root = trust_observation::get_effective_value(INTEL_ROOT_TARGET);
        let sigstore_root = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET);
        intel_root.is_some() && sigstore_root.is_some()
            && hash::sha3_256(intel_root.extract()) == result.intel_root_sha3_256
            && hash::sha3_256(sigstore_root.extract()) == result.sigstore_root_sha3_256
    }

    public fun request_id(self: &VerifiedTinfoilV3Result): u64 { self.request_id }

    public fun verified_at_secs(self: &VerifiedTinfoilV3Result): u64 { self.verified_at_secs }

    public fun intel_root_sha3_256(self: &VerifiedTinfoilV3Result): &vector<u8> {
        &self.intel_root_sha3_256
    }

    public fun sigstore_root_sha3_256(self: &VerifiedTinfoilV3Result): &vector<u8> {
        &self.sigstore_root_sha3_256
    }

    public fun claims(self: &VerifiedTinfoilV3Result): &VerifiedTinfoilV3Claims { &self.claims }

    public fun tls_spki_sha256(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.tls_spki_sha256 }

    public fun hpke_key(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.hpke_key }

    public fun release_digest(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.release_digest }

    public fun trusted_repo(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.trusted_repo }

    public fun nonce(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.nonce }

    public fun mrtd(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.mrtd }

    public fun rtmr0(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr0 }

    public fun rtmr1(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr1 }

    public fun rtmr2(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr2 }

    public fun rtmr3(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.rtmr3 }

    public fun tcb_status(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.tcb_status }

    public fun release_tag(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.release_tag }

    public fun release_commit(self: &VerifiedTinfoilV3Claims): &vector<u8> { &self.release_commit }

    public fun platform_measurement_id(self: &VerifiedTinfoilV3Claims): &vector<u8> {
        &self.platform_measurement_id
    }

    fun assert_open_stage(owner: address, stage: u8) {
        assert!(exists<State>(owner) && State[owner].pending.is_some(),
            error::invalid_state(ENO_PENDING_REQUEST));
        let pending = State[owner].pending.borrow();
        assert!(!pending.closed && pending.stage == stage,
            error::invalid_state(EWRONG_STAGE));
        assert!(timestamp::now_seconds() <= pending.expires_at_secs,
            error::invalid_state(EEXPIRED_REQUEST));
        assert!(is_repository_approved(pending.trusted_repo),
            error::permission_denied(EUNAPPROVED_REPOSITORY));
        assert!(pending.stage_bytes > 0 && pending.stage_bytes <= MAX_STAGE_BYTES,
            error::invalid_state(EINVALID_REQUEST));
    }

    fun assert_quote_claims(pending: &PendingRequest, quote: &QuoteStage) {
        assert!(quote.report_data.length() == 64
            && quote.tls_spki_sha256.length() == 32
            && quote.hpke_key.length() == 32
            && quote.quote_sha256.length() == 32
            && quote.mrtd.length() == 48
            && quote.rtmr0.length() == 48
            && quote.rtmr1.length() == 48
            && quote.rtmr2.length() == 48
            && quote.rtmr3.length() == 48
            && quote.tls_spki_sha256 == pending.expected_tls_spki_sha256,
            error::invalid_state(EINVALID_VERIFIED_CLAIMS));
    }

    fun assert_code_claims(pending: &PendingRequest, code: &CodeStage) {
        assert!(code.repo == pending.trusted_repo && code.digest.length() == 32
            && code.tag.length() > 0 && code.commit.length() == 40,
            error::invalid_state(EINVALID_VERIFIED_CLAIMS));
    }

    fun effective_roots(): (vector<u8>, vector<u8>) {
        let intel = trust_observation::get_effective_value(INTEL_ROOT_TARGET);
        let sigstore = trust_observation::get_effective_value(SIGSTORE_ROOT_TARGET);
        assert!(intel.is_some() && sigstore.is_some(),
            error::invalid_state(ETRUST_ROOT_UNAVAILABLE));
        let intel = intel.extract();
        let sigstore = sigstore.extract();
        assert!(!intel.is_empty() && !sigstore.is_empty(),
            error::invalid_state(ETRUST_ROOT_UNAVAILABLE));
        (intel, sigstore)
    }

    fun checked_roots(pending: &PendingRequest): (vector<u8>, vector<u8>) {
        let (intel, sigstore) = effective_roots();
        assert!(hash::sha3_256(copy intel) == pending.intel_root_sha3_256
            && hash::sha3_256(copy sigstore) == pending.sigstore_root_sha3_256,
            error::invalid_state(ETRUST_ROOT_CHANGED));
        (intel, sigstore)
    }

    fun current_fragment(pending: &PendingRequest): vector<u8> {
        let fragment = vector[];
        let index = pending.stage_start_chunk;
        while (index < pending.chunk_count) {
            fragment.append(*pending.chunks.borrow(index));
            index += 1;
        };
        assert!(fragment.length() == pending.stage_bytes,
            error::invalid_state(EINVALID_REQUEST));
        fragment
    }

    fun advance_stage(pending: &mut PendingRequest, next: u8) {
        pending.stage = next;
        pending.stage_start_chunk = pending.chunk_count;
        pending.stage_bytes = 0;
    }

    fun destroy_empty_pending(pending: PendingRequest) {
        let PendingRequest {
            request_id: _,
            nonce: _,
            trusted_repo: _,
            expected_tls_spki_sha256: _,
            expires_at_secs: _,
            chunks,
            chunk_count,
            total_bytes: _,
            stage: _,
            stage_start_chunk: _,
            stage_bytes: _,
            proof_time_secs: _,
            intel_root_sha3_256: _,
            sigstore_root_sha3_256: _,
            quote: _,
            code: _,
            platform: _,
            closed: _,
            cleanup_cursor,
        } = pending;
        assert!(cleanup_cursor == chunk_count, error::invalid_state(EINVALID_REQUEST));
        chunks.destroy_empty();
    }

    native fun verify_tinfoil_v3_quote(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        intel_root_der: vector<u8>,
    ): QuoteStage;

    native fun verify_tinfoil_v3_code(
        fragment: vector<u8>,
        nonce: vector<u8>,
        trusted_repo: vector<u8>,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
    ): CodeStage;

    native fun verify_tinfoil_v3_code_freshness(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        code: CodeStage,
    );

    native fun verify_tinfoil_v3_platform(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        code: CodeStage,
    ): PlatformStage;

    native fun verify_tinfoil_v3_platform_freshness(
        fragment: vector<u8>,
        nonce: vector<u8>,
        proof_time_secs: u64,
        sigstore_root_json: vector<u8>,
        quote: QuoteStage,
        platform: PlatformStage,
    );

    #[test_only]
    fun filled_byte(value: u8, count: u64): vector<u8> {
        let bytes = vector[];
        for (_i in 0..count) { bytes.push_back(value); };
        bytes
    }

    #[test_only]
    /// The former challenge preimage, retained to catch a return to a
    /// predictable nonce derived solely from public request fields.
    struct PredictableChallengeMaterial has drop {
        domain: vector<u8>,
        chain_id: u8,
        owner: address,
        request_id: u64,
    }

    #[test_only]
    fun setup_roots_for_test(framework: &signer) {
        aptos_framework::account::create_account_for_test(@aptos_framework);
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        aptos_framework::config_buffer::initialize(framework);
        aptos_framework::reconfiguration::initialize_for_test(framework);
        trust_observation::initialize(framework);
        trust_observation::upsert_target_for_next_epoch(
            framework, INTEL_ROOT_TARGET, std::string::utf8(b"https://intel.example/root.cer"), 1, 4096,
        );
        trust_observation::upsert_target_for_next_epoch(
            framework, SIGSTORE_ROOT_TARGET, std::string::utf8(b"https://sigstore.example/trusted_root.json"), 2, 4096,
        );
        trust_observation::activate_for_test(framework);
        trust_observation::set_replacement_patch(framework, INTEL_ROOT_TARGET, b"intel root v1");
        trust_observation::set_replacement_patch(framework, SIGSTORE_ROOT_TARGET, b"sigstore root v1");
        set_repository_approved(framework, b"tinfoilsh/example", true);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_nonce_requires_unpredictable_entropy(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        let predictable = hash::sha3_256(bcs::to_bytes(&PredictableChallengeMaterial {
            domain: CHALLENGE_DOMAIN,
            chain_id: chain_id::get(),
            owner: owner.address_of(),
            request_id: 0,
        }));
        let issued = begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        assert!(issued != predictable, 1);
        assert!(pending_nonce(owner.address_of()).extract() == issued, 2);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000C, location = Self)]
    fun test_reject_stage_out_of_order(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"untrusted code fragment");
        verify_code(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_revocation_blocks_in_flight_request(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"untrusted quote fragment");
        set_repository_approved(framework, b"tinfoilsh/example", false);
        verify_quote(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000D, location = Self)]
    fun test_root_rotation_blocks_later_stage(framework: &signer, owner: &signer) {
        setup_roots_for_test(framework);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"untrusted code fragment");
        // Simulate the quote stage having pinned the old roots. No proof output
        // is fabricated: verify_code aborts at the root check before native use.
        let pending = State[owner.address_of()].pending.borrow_mut();
        pending.stage = STAGE_CODE;
        pending.intel_root_sha3_256 = hash::sha3_256(b"intel root v1");
        pending.sigstore_root_sha3_256 = hash::sha3_256(b"sigstore root v1");
        trust_observation::set_replacement_patch(framework, INTEL_ROOT_TARGET, b"intel root v2");
        verify_code(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000A, location = Self)]
    fun test_quote_key_must_match_request(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        let quote = QuoteStage {
            report_data: filled_byte(0, 64),
            tls_spki_sha256: filled_byte(2, 32),
            hpke_key: filled_byte(3, 32),
            quote_sha256: filled_byte(4, 32),
            mrtd: filled_byte(0, 48),
            rtmr0: filled_byte(0, 48),
            rtmr1: filled_byte(0, 48),
            rtmr2: filled_byte(0, 48),
            rtmr3: filled_byte(0, 48),
            tcb_status: b"UpToDate",
            ppid: filled_byte(0, 16),
            qe_vendor_id: filled_byte(0, 16),
            tee_tcb_svn: filled_byte(0, 16),
            mr_seam: filled_byte(0, 48),
            td_attributes: filled_byte(0, 8),
            xfam: filled_byte(0, 8),
            tcb_evaluation_data_number: 1,
        };
        assert_quote_claims(State[owner.address_of()].pending.borrow(), &quote);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = 0x3000A, location = Self)]
    fun test_code_repo_must_match_request(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        let code = CodeStage {
            repo: b"attacker/other", tag: b"v1", commit: filled_byte(97, 40),
            subject_name: b"image", digest: filled_byte(2, 32),
            shape_cpus: 1, shape_memory_mb: 1024, shape_gpus: 0, shape_disks: 1,
        };
        assert_code_claims(State[owner.address_of()].pending.borrow(), &code);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    #[expected_failure(abort_code = EINVALID_TINFOIL_PROOF, location = Self)]
    fun test_reject_malformed_quote_fragment(framework: &signer, owner: &signer) {
        setup_roots_for_test(framework);
        begin(owner, b"tinfoilsh/example", filled_byte(1, 32), 60);
        append_chunk(owner, b"malformed v3 fragment");
        verify_quote(owner);
    }

    #[test(framework = @aptos_framework, owner = @0x42)]
    fun test_chunked_lifecycle(framework: &signer, owner: &signer) {
        chain_id::initialize_for_test(framework, 4);
        timestamp::set_time_has_started_for_testing(framework);
        randomness::initialize_for_testing(framework);
        set_repository_approved(framework, b"tinfoilsh/example", true);
        let tls_spki = vector[];
        for (i in 0..32) { tls_spki.push_back(1); };
        let first = begin(owner, b"tinfoilsh/example", copy tls_spki, 60);
        assert!(first.length() == 32, 1);
        append_chunk(owner, b"first");
        append_chunk(owner, b"second");
        for (_i in 0..8) { append_chunk(owner, b"x"); };
        assert!(State[owner.address_of()].pending.borrow().total_bytes == 19, 2);
        cancel(owner);
        assert!(pending_nonce(owner.address_of()).is_none(), 3);
        assert!(State[owner.address_of()].pending.is_some(), 4);
        clean_up(owner.address_of(), 8);
        let second = begin(owner, b"tinfoilsh/example", tls_spki, 60);
        assert!(first != second, 5);
        set_repository_approved(framework, b"tinfoilsh/example", false);
        assert!(!is_repository_approved(b"tinfoilsh/example"), 6);
    }
}
