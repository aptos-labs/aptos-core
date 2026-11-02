/// Cross-chain message endpoint, shaped after LayerZero V2 on Aptos.
///
/// Per-message compute is deliberately near zero. Payload bytes arrive from
/// the caller, and a message costs an argument decode, a nonce compare, and a
/// table write, so what a number from this workload moves with is the storage
/// IO a relay leaves behind rather than anything the interpreter does.
///
/// Every `bench_` entry folds a caller-supplied channel id into the opened
/// range, clamps its counts, and returns without writing when the channel has
/// no work. None of them can abort.
module bench::relay_endpoint {
    use std::signer;
    use std::vector;
    use bench::relay_channel;
    use bench::relay_dvn;
    use bench::relay_executor;
    use bench::relay_msglib;
    use bench::relay_payload_store;
    use bench::relay_registry;

    /// Only the package address may run the admin-signed setup.
    const E_NOT_BENCH: u64 = 1;

    /// Message library version every channel is registered on.
    const MSGLIB_VERSION: u8 = 1;

    /// Largest count a `bench_` entry will act on, whatever it was handed.
    const MAX_MSGS: u64 = 64;

    /// Keeps the read-only fold away from an overflow without bounding what it
    /// reads.
    const FOLD_MOD: u64 = 1000000007;

    // Setup, publisher-signed.

    public entry fun initialize(
        admin: &signer, n_channels: u64, src_eid: u32, dst_eid: u32
    ) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        relay_registry::initialize(admin);
        relay_channel::initialize(admin, n_channels, src_eid, dst_eid);
        relay_dvn::initialize(admin);
        relay_executor::initialize(admin);
        relay_payload_store::initialize(admin);
    }

    /// Register the OApps for channels `[start, start + count)` and open their
    /// channels. Callers chunk this: opening every channel of a large run in
    /// one transaction runs past the per-transaction execution limit.
    public entry fun register_oapps(admin: &signer, start: u64, count: u64) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let (_, dst_eid) = relay_channel::eids();
        let i = 0;
        while (i < count) {
            let channel_id = start + i;
            relay_registry::register(
                admin, channel_id, dst_eid, MSGLIB_VERSION);
            relay_channel::open(
                admin, channel_id, relay_registry::derive_peer(channel_id));
            i = i + 1;
        }
    }

    // Onboarding, one transaction per account.

    /// Point the account's channel at it and prepay the executor, so no mix
    /// branch later finds an unfunded channel.
    public entry fun bench_onboard(
        user: &signer, channel_id: u64, prepay: u128
    ) {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return;
        relay_channel::attach(id, signer::address_of(user));
        relay_executor::prepay(id, prepay);
    }

    // The mix.

    /// Send every payload handed over. There is no cap: the payload vector is
    /// the workload, and its shape is the knob.
    public entry fun bench_send(
        _user: &signer, channel_id: u64, payloads: vector<vector<u8>>
    ) {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return;
        let (src_eid, dst_eid) = relay_channel::eids();
        let n = vector::length(&payloads);
        // Popping from the back moves each payload out instead of copying it.
        vector::reverse(&mut payloads);
        let bytes = 0;
        let i = 0;
        while (i < n) {
            let payload = vector::pop_back(&mut payloads);
            let nonce = relay_channel::next_outbound(id);
            let guid = relay_msglib::guid(src_eid, dst_eid, id, nonce);
            bytes = bytes + vector::length(&payload);
            relay_payload_store::put(
                relay_payload_store::slot(
                    relay_msglib::guid_slot_bits(guid), false),
                payload,
            );
            i = i + 1;
        };
        relay_executor::record_send(id, n, bytes);
    }

    /// Deliver the lesser of the payloads handed over and what the channel has
    /// in flight. A channel with nothing outstanding delivers nothing and
    /// writes nothing.
    public entry fun bench_deliver(
        _user: &signer, channel_id: u64, payloads: vector<vector<u8>>
    ) {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return;
        let (src_eid, dst_eid) = relay_channel::eids();
        let available = relay_channel::outstanding(id);
        let n = vector::length(&payloads);
        if (n > available) n = available;
        vector::reverse(&mut payloads);
        let bytes = 0;
        let delivered = 0;
        // Attestations are never consulted: a delivery that runs ahead of the
        // verifiers still has to land, so reading the DVN here would be a cold
        // slot per message that changes nothing.
        while (delivered < n) {
            let payload = vector::pop_back(&mut payloads);
            let nonce = relay_channel::advance_inbound(id);
            let guid = relay_msglib::guid(src_eid, dst_eid, id, nonce);
            bytes = bytes + vector::length(&payload);
            relay_payload_store::put(
                relay_payload_store::slot(
                    relay_msglib::guid_slot_bits(guid), true),
                payload,
            );
            delivered = delivered + 1;
        };
        relay_executor::record_delivery(id, delivered, bytes);
    }

    /// Attest the next `n_msgs` GUIDs on the channel. Nothing is checked
    /// against the outbound nonce, so a GUID that was already attested just
    /// takes the new count.
    public entry fun bench_verify(
        _user: &signer,
        channel_id: u64,
        n_msgs: u64,
        verifiers_per_msg: u64,
    ) {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return;
        let n = if (n_msgs > MAX_MSGS) MAX_MSGS else n_msgs;
        let configured = relay_dvn::verifier_count(id);
        let count =
            if (verifiers_per_msg > configured) configured
            else verifiers_per_msg;
        let (src_eid, dst_eid) = relay_channel::eids();
        let base = relay_channel::inbound(id);
        let i = 0;
        while (i < n) {
            relay_dvn::attest(
                relay_msglib::guid(src_eid, dst_eid, id, base + i + 1), count);
            i = i + 1;
        }
    }

    /// Read the channel, its registry entry, its verifier set, its executor
    /// account, and a stored payload, `depth` times over, writing nothing.
    public entry fun bench_read_state(
        _user: &signer, channel_id: u64, depth: u64
    ) {
        read_state(channel_id, depth);
    }

    /// Skip stuck messages: consume their nonces and delete their stored
    /// payloads. `n_msgs` is clamped to what is in flight, so a channel with
    /// nothing outstanding is a no-op.
    public entry fun bench_skip(
        _user: &signer, channel_id: u64, n_msgs: u64
    ) {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return;
        let outstanding = relay_channel::outstanding(id);
        let n = if (n_msgs > outstanding) outstanding else n_msgs;
        if (n > MAX_MSGS) n = MAX_MSGS;
        let (src_eid, dst_eid) = relay_channel::eids();
        let i = 0;
        while (i < n) {
            let nonce = relay_channel::advance_inbound(id);
            let guid = relay_msglib::guid(src_eid, dst_eid, id, nonce);
            relay_payload_store::drop_slot(
                relay_payload_store::slot(
                    relay_msglib::guid_slot_bits(guid), false));
            i = i + 1;
        };
        relay_channel::note_skipped(id, n);
        relay_executor::record_skip(id, n);
    }

    /// Repoint the channel's message library, verifier set, and executor fee.
    /// Every field is clamped rather than validated, since the mix drives this
    /// from an unprivileged account.
    public entry fun bench_set_config(
        _user: &signer,
        channel_id: u64,
        verifiers_per_msg: u64,
        msglib_version: u8,
        fee_per_byte: u64,
    ) {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return;
        relay_registry::set_msglib(id, msglib_version);
        relay_dvn::resize(id, verifiers_per_msg);
        relay_executor::set_fee_per_byte(id, fee_per_byte);
    }

    // Helpers and views.

    /// Fold a caller-supplied channel id into the opened range, so an id past
    /// the end picks a channel instead of aborting.
    fun normalize(channel_id: u64): u64 {
        let n = relay_channel::declared_count();
        if (n == 0) 0 else channel_id % n
    }

    /// The read-only fold `bench_read_state` runs, exposed so a test can check
    /// it touched something.
    public fun read_state(channel_id: u64, depth: u64): u64 {
        let id = normalize(channel_id);
        if (!relay_channel::is_open(id)) return 0;
        let n = if (depth > MAX_MSGS) MAX_MSGS else depth;
        let (src_eid, dst_eid) = relay_channel::eids();
        let base = relay_channel::inbound(id);
        let acc = 0;
        let i = 0;
        while (i < n) {
            let guid =
                relay_msglib::guid(src_eid, dst_eid, id, base + i + 1);
            let slot =
                relay_payload_store::slot(
                    relay_msglib::guid_slot_bits(guid), false);
            acc = (acc
                + relay_channel::outbound(id)
                + relay_dvn::attestations(guid)
                + relay_dvn::verifier_count(id)
                + (relay_registry::msglib_version_of(id) as u64)
                + relay_payload_store::payload_len(slot)
                + relay_executor::messages_sent(id)) % FOLD_MOD;
            i = i + 1;
        };
        acc
    }

    #[view]
    public fun channel_count(): u64 {
        relay_channel::declared_count()
    }

    #[view]
    public fun outbound_nonce(channel_id: u64): u64 {
        relay_channel::outbound(normalize(channel_id))
    }

    #[view]
    public fun inbound_nonce(channel_id: u64): u64 {
        relay_channel::inbound(normalize(channel_id))
    }

    #[view]
    /// Whether the payload of `nonce` is still stored on the given side of the
    /// channel.
    public fun has_payload(
        channel_id: u64, nonce: u64, inbound: bool
    ): bool {
        let id = normalize(channel_id);
        let (src_eid, dst_eid) = relay_channel::eids();
        let guid = relay_msglib::guid(src_eid, dst_eid, id, nonce);
        relay_payload_store::contains(
            relay_payload_store::slot(
                relay_msglib::guid_slot_bits(guid), inbound))
    }

    #[view]
    public fun attestations_for(channel_id: u64, nonce: u64): u64 {
        let id = normalize(channel_id);
        let (src_eid, dst_eid) = relay_channel::eids();
        relay_dvn::attestations(
            relay_msglib::guid(src_eid, dst_eid, id, nonce))
    }

    #[view]
    public fun msglib_version(): u8 { MSGLIB_VERSION }

    #[view]
    public fun max_msgs(): u64 { MAX_MSGS }
}
