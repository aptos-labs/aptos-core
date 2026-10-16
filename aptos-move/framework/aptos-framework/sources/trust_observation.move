/// Validator-observed external trust material. Governance chooses which HTTPS
/// resources validators observe; the observer canonicalizes the response, and a
/// quorum-certified validator transaction updates `ObservedTrustData`.
/// Governance patches are kept separate so a new observation cannot erase one.
module aptos_framework::trust_observation {
    use std::error;
    use std::option;
    use std::option::Option;
    use std::string::String;
    use aptos_std::comparator::compare_u8_vector;
    use aptos_framework::config_buffer;
    use aptos_framework::event::emit;
    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    #[test_only]
    use aptos_framework::account::create_account_for_test;

    friend aptos_framework::reconfiguration_with_dkg;

    const EINVALID_TARGET: u64 = 1;
    const ETOO_MANY_TARGETS: u64 = 2;
    const EUNKNOWN_TARGET: u64 = 3;
    const EUNEXPECTED_CONFIG_VERSION: u64 = 4;
    const EUNEXPECTED_OBSERVED_VERSION: u64 = 5;
    const EINVALID_VALUE: u64 = 6;
    const EUNEXPECTED_EPOCH: u64 = 7;

    const FORMAT_DER_CERTIFICATE: u8 = 1;
    const FORMAT_CANONICAL_JSON: u8 = 2;
    const FORMAT_RAW_BYTES: u8 = 3;
    const PATCH_REPLACE: u8 = 1;
    const PATCH_REMOVE: u8 = 2;

    const MAX_TARGETS: u64 = 16;
    const MAX_TARGET_ID_BYTES: u64 = 64;
    const MAX_TARGET_URL_BYTES: u64 = 2048;
    // 16 targets at this cap leave room below the default 1 MiB per-resource
    // write limit for BCS lengths, IDs, versions, and resource metadata.
    const MAX_VALUE_BYTES: u64 = 60 * 1024;

    /// Sorted by `id`. Configured through governance and applied at epoch change.
    struct Config has copy, drop, key, store {
        targets: vector<TargetSpec>,
    }

    /// The format is interpreted by the validator's versioned canonicalizer.
    /// A config edit increments `config_version`, invalidating old observations.
    struct TargetSpec has copy, drop, store {
        id: vector<u8>,
        url: String,
        format: u8,
        max_bytes: u64,
        config_version: u64,
    }

    /// BCS layout shared with the validator transaction payload.
    struct ObservedValue has copy, drop, store {
        target_id: vector<u8>,
        config_version: u64,
        epoch: u64,
        version: u64,
        value: vector<u8>,
    }

    /// Only the validator transaction path should change this resource.
    /// Entries are sorted by `target_id` and bounded by `MAX_TARGETS`.
    struct ObservedTrustData has copy, drop, key, store {
        entries: vector<ObservedValue>,
    }

    #[event]
    /// Emitted after a quorum-certified observation is committed. The full
    /// value lets observers resynchronize without an extra state read.
    struct ObservedTrustDataUpdated has drop, store {
        observed: ObservedValue,
    }

    /// A governance override bound to exactly one target configuration.
    /// `PATCH_REMOVE` suppresses the target's effective value; `PATCH_REPLACE`
    /// supplies an effective value even before the first observation.
    struct Patch has copy, drop, store {
        target_id: vector<u8>,
        config_version: u64,
        kind: u8,
        value: vector<u8>,
    }

    struct Patches has key {
        entries: vector<Patch>,
    }

    /// The only trust material that verifiers should consume.
    struct EffectiveTrustData has key {
        entries: vector<ObservedValue>,
    }

    /// Called once during genesis, or once by governance when upgrading an
    /// existing chain. Starts with no trusted external targets or values.
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        move_to(framework, Config { targets: vector[] });
        move_to(framework, ObservedTrustData { entries: vector[] });
        move_to(framework, Patches { entries: vector[] });
        move_to(framework, EffectiveTrustData { entries: vector[] });
    }

    /// Governance stages a new target or changes an existing one for the next
    /// epoch. `format` is 1 for a DER certificate, 2 for canonical JSON, or
    /// 3 for raw bytes. The format-specific validator code decides how to
    /// canonicalize and validate the downloaded response.
    /// `config_version` is assigned here, never accepted from the caller.
    public fun upsert_target_for_next_epoch(
        framework: &signer,
        id: vector<u8>,
        url: String,
        format: u8,
        max_bytes: u64,
    ) {
        system_addresses::assert_aptos_framework(framework);
        assert!(id.length() > 0 && id.length() <= MAX_TARGET_ID_BYTES,
            error::invalid_argument(EINVALID_TARGET));
        assert!(url.bytes().length() > 8 && url.bytes().length() <= MAX_TARGET_URL_BYTES
            && is_https_url(&url),
            error::invalid_argument(EINVALID_TARGET));
        assert!(format == FORMAT_DER_CERTIFICATE || format == FORMAT_CANONICAL_JSON || format == FORMAT_RAW_BYTES,
            error::invalid_argument(EINVALID_TARGET));
        assert!(max_bytes > 0 && max_bytes <= MAX_VALUE_BYTES,
            error::invalid_argument(EINVALID_TARGET));

        let config = pending_or_current_config();
        let (found_pending, pending_index) = find_target(&config.targets, &id);
        let (found_current, current_index) = find_target(&Config[@aptos_framework].targets, &id);
        let version = if (found_pending) {
            config.targets[pending_index].config_version + 1
        } else if (found_current) {
            Config[@aptos_framework].targets[current_index].config_version + 1
        } else {
            1
        };
        let target = TargetSpec { id, url, format, max_bytes, config_version: version };
        if (found_pending) {
            config.targets[pending_index] = target;
        } else {
            assert!(config.targets.length() < MAX_TARGETS, error::resource_exhausted(ETOO_MANY_TARGETS));
            config.targets.insert(pending_index, target);
        };
        config_buffer::upsert(config);
    }

    /// Governance stops observing a target at the next epoch. Its observed and
    /// patched values are removed when that epoch begins.
    public fun remove_target_for_next_epoch(framework: &signer, id: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        let config = pending_or_current_config();
        let (found, index) = find_target(&config.targets, &id);
        assert!(found, error::not_found(EUNKNOWN_TARGET));
        config.targets.remove(index);
        config_buffer::upsert(config);
    }

    /// Called by the epoch transition, after the pending configuration becomes
    /// active. Values tied to removed or changed target configurations disappear.
    public(friend) fun on_new_epoch(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!config_buffer::does_exist<Config>()) { return };
        let config = config_buffer::extract_v2<Config>();
        *&mut Config[@aptos_framework] = config;

        let observed = &mut ObservedTrustData[@aptos_framework].entries;
        let i = 0;
        while (i < observed.length()) {
            let entry = &observed[i];
            if (!is_current_target(&entry.target_id, entry.config_version)) {
                observed.remove(i);
            } else {
                i += 1;
            }
        };
        let patches = &mut Patches[@aptos_framework].entries;
        let i = 0;
        while (i < patches.length()) {
            let patch = &patches[i];
            if (!is_current_target(&patch.target_id, patch.config_version)) {
                patches.remove(i);
            } else {
                i += 1;
            }
        };
        regenerate_effective();
    }

    /// Invoked by the quorum-certified validator transaction executor. The
    /// framework signer check is defense in depth; callers must verify quorum
    /// signatures before entering the VM. A direct framework call cannot skip
    /// target, size, configuration-version, or observation-version checks.
    public fun upsert_observed_value(framework: &signer, observed: ObservedValue) {
        system_addresses::assert_aptos_framework(framework);
        let config = &Config[@aptos_framework];
        let (found_target, target_index) = find_target(&config.targets, &observed.target_id);
        assert!(found_target, error::not_found(EUNKNOWN_TARGET));
        let target = &config.targets[target_index];
        assert!(observed.config_version == target.config_version,
            error::invalid_argument(EUNEXPECTED_CONFIG_VERSION));
        assert!(observed.epoch == reconfiguration::current_epoch(),
            error::invalid_argument(EUNEXPECTED_EPOCH));
        assert!(observed.value.length() > 0 && observed.value.length() <= target.max_bytes,
            error::invalid_argument(EINVALID_VALUE));

        let entries = &mut ObservedTrustData[@aptos_framework].entries;
        let (found, index) = find_observed(entries, &observed.target_id);
        let expected_version = if (found) { entries[index].version + 1 } else { 1 };
        assert!(observed.version == expected_version,
            error::invalid_argument(EUNEXPECTED_OBSERVED_VERSION));
        if (found) {
            entries[index] = copy observed;
        } else {
            entries.insert(index, copy observed);
        };
        emit(ObservedTrustDataUpdated { observed });
        regenerate_effective();
    }

    /// Governance replaces a target's effective value until the patch is
    /// explicitly cleared or its target configuration changes. Observation
    /// continues and can be inspected independently while the patch is active.
    public fun set_replacement_patch(framework: &signer, target_id: vector<u8>, value: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        let target = current_target(&target_id);
        assert!(value.length() > 0 && value.length() <= target.max_bytes,
            error::invalid_argument(EINVALID_VALUE));
        set_patch(Patch { target_id, config_version: target.config_version, kind: PATCH_REPLACE, value });
    }

    /// Governance suppresses a target's effective value until the patch is
    /// explicitly cleared or its target configuration changes.
    public fun set_removal_patch(framework: &signer, target_id: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        let target = current_target(&target_id);
        set_patch(Patch {
            target_id,
            config_version: target.config_version,
            kind: PATCH_REMOVE,
            value: vector[],
        });
    }

    /// Governance clears an override and restores the latest observed value.
    public fun clear_patch(framework: &signer, target_id: vector<u8>) {
        system_addresses::assert_aptos_framework(framework);
        let entries = &mut Patches[@aptos_framework].entries;
        let (found, index) = find_patch(entries, &target_id);
        if (found) { entries.remove(index); };
        regenerate_effective();
    }

    /// The verifier-facing API. `none` means that there is no currently trusted
    /// material for the target (including when a removal patch is active).
    public fun get_effective_value(target_id: vector<u8>): Option<vector<u8>> {
        let entries = &EffectiveTrustData[@aptos_framework].entries;
        let (found, index) = find_observed(entries, &target_id);
        if (found) { option::some(entries[index].value) } else { option::none() }
    }

    /// For observer configuration and audit; verifiers use `get_effective_value`.
    public fun get_targets(): vector<TargetSpec> {
        Config[@aptos_framework].targets
    }

    /// For diagnostics; this ignores governance overrides.
    public fun get_observed_value(target_id: vector<u8>): Option<ObservedValue> {
        let entries = &ObservedTrustData[@aptos_framework].entries;
        let (found, index) = find_observed(entries, &target_id);
        if (found) { option::some(entries[index]) } else { option::none() }
    }

    #[test_only]
    // Lets verifier unit tests apply a staged target without initializing the
    // entire validator-set/DKG epoch transition. This is absent in production.
    public fun activate_for_test(framework: &signer) {
        on_new_epoch(framework);
    }

    fun pending_or_current_config(): Config {
        if (config_buffer::does_exist<Config>()) {
            config_buffer::extract_v2<Config>()
        } else {
            Config[@aptos_framework]
        }
    }

    fun current_target(id: &vector<u8>): TargetSpec {
        let config = &Config[@aptos_framework];
        let (found, index) = find_target(&config.targets, id);
        assert!(found, error::not_found(EUNKNOWN_TARGET));
        config.targets[index]
    }

    fun is_current_target(id: &vector<u8>, config_version: u64): bool {
        let config = &Config[@aptos_framework];
        let (found, index) = find_target(&config.targets, id);
        found && config.targets[index].config_version == config_version
    }

    fun set_patch(patch: Patch) {
        let entries = &mut Patches[@aptos_framework].entries;
        let (found, index) = find_patch(entries, &patch.target_id);
        if (found) { entries[index] = patch; } else { entries.insert(index, patch); };
        regenerate_effective();
    }

    fun regenerate_effective() {
        let targets = &Config[@aptos_framework].targets;
        let observed = &ObservedTrustData[@aptos_framework].entries;
        let patches = &Patches[@aptos_framework].entries;
        let effective = vector[];
        let i = 0;
        while (i < targets.length()) {
            let target = &targets[i];
            let (has_patch, patch_index) = find_patch(patches, &target.id);
            let (has_observed, observed_index) = find_observed(observed, &target.id);
            if (has_patch) {
                let patch = &patches[patch_index];
                if (patch.kind == PATCH_REPLACE) {
                    let version = if (has_observed) { observed[observed_index].version } else { 0 };
                    effective.push_back(ObservedValue {
                        target_id: target.id,
                        config_version: target.config_version,
                        epoch: if (has_observed) { observed[observed_index].epoch }
                            else { reconfiguration::current_epoch() },
                        version,
                        value: patch.value,
                    });
                };
            } else if (has_observed) {
                effective.push_back(observed[observed_index]);
            };
            i += 1;
        };
        EffectiveTrustData[@aptos_framework].entries = effective;
    }

    /// Return `(found, insertion_index)` in a vector sorted by target ID.
    fun find_target(entries: &vector<TargetSpec>, id: &vector<u8>): (bool, u64) {
        let i = 0;
        while (i < entries.length()) {
            let comparison = compare_u8_vector(*id, entries[i].id);
            if (comparison.is_equal()) { return (true, i) };
            if (comparison.is_smaller_than()) { return (false, i) };
            i += 1;
        };
        (false, i)
    }

    fun find_observed(entries: &vector<ObservedValue>, id: &vector<u8>): (bool, u64) {
        let i = 0;
        while (i < entries.length()) {
            let comparison = compare_u8_vector(*id, entries[i].target_id);
            if (comparison.is_equal()) { return (true, i) };
            if (comparison.is_smaller_than()) { return (false, i) };
            i += 1;
        };
        (false, i)
    }

    fun find_patch(entries: &vector<Patch>, id: &vector<u8>): (bool, u64) {
        let i = 0;
        while (i < entries.length()) {
            let comparison = compare_u8_vector(*id, entries[i].target_id);
            if (comparison.is_equal()) { return (true, i) };
            if (comparison.is_smaller_than()) { return (false, i) };
            i += 1;
        };
        (false, i)
    }

    fun is_https_url(url: &String): bool {
        let bytes = url.bytes();
        let prefix = b"https://";
        if (bytes.length() <= prefix.length()) { return false };
        let i = 0;
        while (i < prefix.length()) {
            if (bytes[i] != prefix[i]) { return false };
            i += 1;
        };
        true
    }

    #[test(framework = @aptos_framework)]
    fun test_observe_patch_and_reconfigure(framework: &signer) {
        create_account_for_test(@aptos_framework);
        config_buffer::initialize(framework);
        reconfiguration::initialize_for_test(framework);
        initialize(framework);
        upsert_target_for_next_epoch(framework, b"intel", std::string::utf8(b"https://intel.example/root.cer"),
            FORMAT_DER_CERTIFICATE, 1024);
        assert!(get_targets().is_empty(), 1);
        on_new_epoch(framework);
        assert!(get_targets().length() == 1, 2);

        let first = ObservedValue {
            target_id: b"intel", config_version: 1, epoch: reconfiguration::current_epoch(),
            version: 1, value: b"DER1",
        };
        upsert_observed_value(framework, first);
        assert!(get_effective_value(b"intel").extract() == b"DER1", 3);
        set_replacement_patch(framework, b"intel", b"DER2");
        assert!(get_effective_value(b"intel").extract() == b"DER2", 4);
        upsert_observed_value(framework, ObservedValue {
            target_id: b"intel", config_version: 1, epoch: reconfiguration::current_epoch(),
            version: 2, value: b"DER3",
        });
        assert!(get_observed_value(b"intel").extract().version == 2, 5);
        assert!(get_effective_value(b"intel").extract() == b"DER2", 6);
        set_removal_patch(framework, b"intel");
        assert!(get_effective_value(b"intel").is_none(), 7);
        clear_patch(framework, b"intel");
        assert!(get_effective_value(b"intel").extract() == b"DER3", 8);

        upsert_target_for_next_epoch(framework, b"intel", std::string::utf8(b"https://intel.example/new-root.cer"),
            FORMAT_DER_CERTIFICATE, 1024);
        on_new_epoch(framework);
        assert!(get_effective_value(b"intel").is_none(), 9);
        assert!(get_observed_value(b"intel").is_none(), 10);
        upsert_observed_value(framework, ObservedValue {
            target_id: b"intel", config_version: 2, epoch: reconfiguration::current_epoch(),
            version: 1, value: b"DER4",
        });
        set_replacement_patch(framework, b"intel", b"DER5");
        remove_target_for_next_epoch(framework, b"intel");
        on_new_epoch(framework);
        assert!(get_effective_value(b"intel").is_none(), 11);
        assert!(get_targets().is_empty(), 12);
        upsert_target_for_next_epoch(framework, b"intel", std::string::utf8(b"https://intel.example/third-root.cer"),
            FORMAT_DER_CERTIFICATE, 1024);
        on_new_epoch(framework);
        assert!(get_effective_value(b"intel").is_none(), 13);
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10005, location = Self)]
    fun test_reject_skipped_observation_version(framework: &signer) {
        create_account_for_test(@aptos_framework);
        config_buffer::initialize(framework);
        reconfiguration::initialize_for_test(framework);
        initialize(framework);
        upsert_target_for_next_epoch(framework, b"t", std::string::utf8(b"https://example.test/root"),
            FORMAT_CANONICAL_JSON, 128);
        on_new_epoch(framework);
        upsert_observed_value(framework, ObservedValue {
            target_id: b"t", config_version: 1, epoch: reconfiguration::current_epoch(),
            version: 2, value: b"{}",
        });
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x10007, location = Self)]
    fun test_reject_observation_from_other_epoch(framework: &signer) {
        create_account_for_test(@aptos_framework);
        config_buffer::initialize(framework);
        reconfiguration::initialize_for_test(framework);
        initialize(framework);
        upsert_target_for_next_epoch(framework, b"t", std::string::utf8(b"https://example.test/root"),
            FORMAT_RAW_BYTES, 128);
        on_new_epoch(framework);
        upsert_observed_value(framework, ObservedValue {
            target_id: b"t", config_version: 1, epoch: reconfiguration::current_epoch() + 1,
            version: 1, value: b"root",
        });
    }

    #[test]
    fun test_https_url_check() {
        assert!(is_https_url(&std::string::utf8(b"https://example.test/root")), 1);
        assert!(!is_https_url(&std::string::utf8(b"http://example.test/root")), 2);
        assert!(!is_https_url(&std::string::utf8(b"https://")), 3);
    }
}
