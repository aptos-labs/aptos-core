#[test_only]
module bench::bridge_relay_tests {
    use std::vector;
    use bench::relay_channel;
    use bench::relay_dvn;
    use bench::relay_endpoint;
    use bench::relay_executor;
    use bench::relay_msglib;
    use bench::relay_payload_store;
    use bench::relay_registry;

    /// Endpoint ids the generator configures, one per side of the bridge.
    const SRC_EID: u32 = 30101;
    const DST_EID: u32 = 30108;

    /// The generator's default knobs.
    const N_CHANNELS: u64 = 512;
    const MSGS_PER_TXN: u64 = 8;
    const PAYLOAD_LEN: u64 = 256;
    const VERIFIERS_PER_MSG: u64 = 2;
    const READ_DEPTH: u64 = 8;

    /// The generator's funding, prepayment, and steady-state config writes.
    const EXECUTOR_FUNDING: u128 = 1000000000000000;
    const ONBOARD_PREPAY: u128 = 1000000000;
    const FEE_PER_BYTE: u64 = 1;
    const MSGLIB_VERSION_ALT: u8 = 2;
    const SKIP_PER_TXN: u64 = 1;

    const OUTBOUND: bool = false;
    const INBOUND: bool = true;

    /// One message body, the width the generator draws. The contents only
    /// have to differ between slots, so they run off the message index.
    fun payload_bytes(index: u64): vector<u8> {
        let bytes = vector::empty<u8>();
        let i = 0;
        while (i < PAYLOAD_LEN) {
            vector::push_back(&mut bytes, (((index + i) % 256) as u8));
            i = i + 1;
        };
        bytes
    }

    fun payloads(): vector<vector<u8>> {
        let messages = vector::empty<vector<u8>>();
        let i = 0;
        while (i < MSGS_PER_TXN) {
            vector::push_back(&mut messages, payload_bytes(i));
            i = i + 1;
        };
        messages
    }

    /// Every publisher-signed call the generator issues, in order.
    fun setup(admin: &signer) {
        relay_endpoint::initialize(admin, N_CHANNELS, SRC_EID, DST_EID);
        relay_endpoint::register_oapps(admin, 0, N_CHANNELS);
        let i = 0;
        while (i < N_CHANNELS) {
            relay_dvn::set_verifiers(admin, i, VERIFIERS_PER_MSG);
            i = i + 1;
        };
        let i = 0;
        while (i < N_CHANNELS) {
            relay_executor::fund(admin, i, EXECUTOR_FUNDING);
            i = i + 1;
        }
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        setup(admin);
        assert!(relay_endpoint::channel_count() == N_CHANNELS, 0);
        assert!(relay_registry::n_oapps() == N_CHANNELS, 0);
        assert!(relay_dvn::verifier_count(0) == VERIFIERS_PER_MSG, 0);
        assert!(relay_executor::balance_of(0) == EXECUTOR_FUNDING, 0);

        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);
        relay_endpoint::bench_onboard(bob, 1, ONBOARD_PREPAY);
        assert!(relay_channel::sender(0) == @0xa11ce, 0);
        assert!(relay_channel::sender(1) == @0xb0b, 0);
        assert!(
            relay_executor::balance_of(1) == EXECUTOR_FUNDING + ONBOARD_PREPAY,
            0,
        );

        relay_endpoint::bench_send(alice, 0, payloads());
        assert!(relay_endpoint::outbound_nonce(0) == MSGS_PER_TXN, 0);
        assert!(relay_endpoint::inbound_nonce(0) == 0, 0);
        assert!(relay_endpoint::has_payload(0, 1, OUTBOUND), 0);
        assert!(relay_endpoint::has_payload(0, MSGS_PER_TXN, OUTBOUND), 0);

        relay_endpoint::bench_verify(alice, 0, MSGS_PER_TXN, VERIFIERS_PER_MSG);
        assert!(relay_endpoint::attestations_for(0, 1) == VERIFIERS_PER_MSG, 0);
        assert!(
            relay_endpoint::attestations_for(0, MSGS_PER_TXN)
                == VERIFIERS_PER_MSG,
            0,
        );

        // Skipping the head leaves one fewer in flight, so the delivery that
        // follows is short of the payloads it was handed.
        relay_endpoint::bench_skip(bob, 0, SKIP_PER_TXN);
        assert!(relay_endpoint::inbound_nonce(0) == SKIP_PER_TXN, 0);
        assert!(!relay_endpoint::has_payload(0, 1, OUTBOUND), 0);
        assert!(relay_executor::messages_skipped(0) == SKIP_PER_TXN, 0);

        relay_endpoint::bench_deliver(bob, 0, payloads());
        assert!(relay_endpoint::inbound_nonce(0) == MSGS_PER_TXN, 0);
        assert!(!relay_endpoint::has_payload(0, 1, INBOUND), 0);
        assert!(relay_endpoint::has_payload(0, 2, INBOUND), 0);
        assert!(relay_endpoint::has_payload(0, MSGS_PER_TXN, INBOUND), 0);

        relay_endpoint::bench_read_state(alice, 0, READ_DEPTH);
        assert!(relay_endpoint::read_state(0, READ_DEPTH) != 0, 0);

        relay_endpoint::bench_set_config(
            alice, 0, VERIFIERS_PER_MSG, MSGLIB_VERSION_ALT, FEE_PER_BYTE
        );
        assert!(relay_dvn::verifier_count(0) == VERIFIERS_PER_MSG, 0);
        assert!(
            relay_registry::msglib_version_of(0) == MSGLIB_VERSION_ALT, 0
        );
        assert!(relay_executor::fee_per_byte_of(0) == FEE_PER_BYTE, 0);

        // Nonces only ever move forward, and the inbound one never passes the
        // outbound one.
        assert!(relay_endpoint::outbound_nonce(0) == MSGS_PER_TXN, 0);
        assert!(
            relay_endpoint::inbound_nonce(0)
                <= relay_endpoint::outbound_nonce(0),
            0,
        );
        assert!(relay_executor::messages_sent(0) == MSGS_PER_TXN, 0);
        assert!(relay_executor::messages_delivered(0) == MSGS_PER_TXN - 1, 0);
        assert!(
            relay_executor::bytes_delivered(0)
                == (MSGS_PER_TXN - 1) * PAYLOAD_LEN,
            0,
        );
        // Delivery never consulted the attestations, so verify's record still
        // stands untouched.
        assert!(relay_endpoint::attestations_for(0, 2) == VERIFIERS_PER_MSG, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_deliver_tolerates_zero_available(
        admin: &signer, alice: &signer
    ) {
        setup(admin);
        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);
        relay_endpoint::bench_deliver(alice, 0, payloads());
        assert!(relay_endpoint::inbound_nonce(0) == 0, 0);
        assert!(!relay_endpoint::has_payload(0, 1, INBOUND), 0);
        assert!(relay_executor::messages_delivered(0) == 0, 0);

        // A channel that ran dry mid-run behaves the same way.
        relay_endpoint::bench_send(alice, 0, vector[b"only"]);
        relay_endpoint::bench_deliver(alice, 0, payloads());
        assert!(relay_endpoint::inbound_nonce(0) == 1, 0);
        relay_endpoint::bench_deliver(alice, 0, payloads());
        assert!(relay_endpoint::inbound_nonce(0) == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_skip_tolerates_past_the_outstanding_count(
        admin: &signer, alice: &signer
    ) {
        setup(admin);
        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);
        relay_endpoint::bench_send(alice, 0, vector[b"a", b"b"]);

        relay_endpoint::bench_skip(alice, 0, 10);
        assert!(relay_endpoint::inbound_nonce(0) == 2, 0);
        assert!(relay_endpoint::outbound_nonce(0) == 2, 0);
        assert!(relay_executor::messages_skipped(0) == 2, 0);
        assert!(!relay_endpoint::has_payload(0, 1, OUTBOUND), 0);
        assert!(!relay_endpoint::has_payload(0, 2, OUTBOUND), 0);

        // Nothing left to skip, so the next one writes nothing.
        relay_endpoint::bench_skip(alice, 0, 10);
        assert!(relay_endpoint::inbound_nonce(0) == 2, 0);
        assert!(relay_executor::messages_skipped(0) == 2, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_verify_tolerates_a_repeat_attestation(
        admin: &signer, alice: &signer
    ) {
        setup(admin);
        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);
        relay_endpoint::bench_verify(alice, 0, 2, 1);
        assert!(relay_endpoint::attestations_for(0, 1) == 1, 0);

        // The same GUIDs again, and the later count is what stands.
        relay_endpoint::bench_verify(alice, 0, 2, VERIFIERS_PER_MSG);
        assert!(relay_endpoint::attestations_for(0, 1) == VERIFIERS_PER_MSG, 0);
        assert!(relay_endpoint::attestations_for(0, 2) == VERIFIERS_PER_MSG, 0);

        // More verifiers than the channel has are clamped, not rejected.
        relay_endpoint::bench_verify(alice, 0, 2, 1000);
        assert!(relay_endpoint::attestations_for(0, 1) == VERIFIERS_PER_MSG, 0);
    }

    #[test(admin = @bench)]
    fun test_attestations_fold_onto_the_ring(admin: &signer) {
        relay_endpoint::initialize(admin, 2, SRC_EID, DST_EID);
        let ring = relay_msglib::guid_ring();

        relay_dvn::attest(relay_msglib::guid(SRC_EID, DST_EID, 0, 1), 3);
        assert!(
            relay_dvn::attestations(
                relay_msglib::guid(SRC_EID, DST_EID, 0, 1)) == 3,
            0,
        );

        // Verify attests ahead of a cursor that only moves forward, so keying
        // on the raw nonce would add an item per message and never reclaim
        // one. A nonce a ring on has to land in the same slot and take it
        // over, which is what holds the table flat for the length of a run.
        relay_dvn::attest(
            relay_msglib::guid(SRC_EID, DST_EID, 0, 1 + ring), 7);
        assert!(
            relay_dvn::attestations(
                relay_msglib::guid(SRC_EID, DST_EID, 0, 1)) == 7,
            0,
        );

        // Channels keep their own rings.
        assert!(
            relay_dvn::attestations(
                relay_msglib::guid(SRC_EID, DST_EID, 1, 1)) == 0,
            0,
        );
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_send_tolerates_an_empty_payload(admin: &signer, alice: &signer) {
        setup(admin);
        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);

        relay_endpoint::bench_send(alice, 0, vector[b"", b"x"]);
        assert!(relay_endpoint::outbound_nonce(0) == 2, 0);
        // A zero-length payload still occupies its slot.
        assert!(relay_endpoint::has_payload(0, 1, OUTBOUND), 0);
        assert!(relay_executor::messages_sent(0) == 2, 0);

        // No payloads at all is a no-op rather than an abort.
        relay_endpoint::bench_send(alice, 0, vector[]);
        assert!(relay_endpoint::outbound_nonce(0) == 2, 0);

        relay_endpoint::bench_deliver(alice, 0, vector[b"", b""]);
        assert!(relay_endpoint::inbound_nonce(0) == 2, 0);
        assert!(relay_executor::bytes_delivered(0) == 0, 0);
    }

    #[test]
    fun test_msglib_guid_framing() {
        let guid = relay_msglib::guid(7, 9, 3, 5);
        let expected =
            (7u128 << 96) | (9u128 << 64) | (3u128 << 32) | 5u128;
        assert!(guid == expected, 0);
        assert!(relay_msglib::guid_channel(guid) == 3, 0);
        assert!(relay_msglib::guid_nonce(guid) == 5, 0);
        assert!(relay_msglib::guid_slot_bits(guid) == (3 << 32) | 5, 0);

        // Each field is 32 bits wide, so anything above that wraps rather
        // than colliding with the field above it.
        let wide = relay_msglib::guid(0, 0, 4294967298, 4294967299);
        assert!(relay_msglib::guid_channel(wide) == 2, 0);
        assert!(relay_msglib::guid_nonce(wide) == 3, 0);

        // Distinct channels and nonces give distinct slots, and the two
        // directions of one message never share one.
        let other = relay_msglib::guid(7, 9, 3, 6);
        let bits = relay_msglib::guid_slot_bits(guid);
        let other_bits = relay_msglib::guid_slot_bits(other);
        assert!(bits != other_bits, 0);
        assert!(
            relay_payload_store::slot(bits, INBOUND)
                == bits | (1 << 63),
            0,
        );
        assert!(relay_payload_store::slot(bits, OUTBOUND) == bits, 0);

        // Nonces run forever but slots do not. The ring is what keeps the
        // store at a fixed size instead of growing for the length of a run,
        // and it has to be wide enough for every message one transaction can
        // name.
        let ring = relay_msglib::guid_ring();
        assert!(ring >= relay_endpoint::max_msgs(), 0);
        assert!(
            relay_msglib::guid_slot_bits(relay_msglib::guid(7, 9, 3, 5 + ring))
                == bits,
            0,
        );
        assert!(
            relay_msglib::guid_slot_bits(
                relay_msglib::guid(7, 9, 4, 5)
            ) != bits,
            0,
        );
    }

    #[test(admin = @bench)]
    fun test_channel_nonce_ordering(admin: &signer) {
        relay_endpoint::initialize(admin, 2, SRC_EID, DST_EID);
        relay_endpoint::register_oapps(admin, 0, 2);

        // Nonces count from one, matching the upstream endpoint.
        assert!(relay_channel::next_outbound(0) == 1, 0);
        assert!(relay_channel::next_outbound(0) == 2, 0);
        assert!(relay_channel::next_outbound(0) == 3, 0);
        // Channels count independently.
        assert!(relay_channel::next_outbound(1) == 1, 0);
        assert!(relay_channel::outstanding(0) == 3, 0);

        assert!(relay_channel::advance_inbound(0) == 1, 0);
        assert!(relay_channel::advance_inbound(0) == 2, 0);
        assert!(relay_channel::outstanding(0) == 1, 0);
        assert!(relay_channel::advance_inbound(0) == 3, 0);
        // The inbound nonce stops at the outbound one instead of running past
        // it, which is what makes a delivery on an empty channel a no-op.
        assert!(relay_channel::advance_inbound(0) == 0, 0);
        assert!(relay_channel::inbound(0) == 3, 0);
        assert!(relay_channel::outstanding(0) == 0, 0);
        assert!(relay_channel::inbound(1) == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_set_config_clamps_verifiers(admin: &signer, alice: &signer) {
        setup(admin);
        relay_endpoint::bench_set_config(alice, 0, 1000, 3, 0);
        assert!(relay_dvn::verifier_count(0) == relay_dvn::max_verifiers(), 0);
        assert!(relay_registry::msglib_version_of(0) == 3, 0);
        // A zero fee leaves the balance where it is rather than underflowing.
        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);
        relay_endpoint::bench_send(alice, 0, payloads());
        assert!(
            relay_executor::balance_of(0)
                == EXECUTOR_FUNDING + ONBOARD_PREPAY,
            0,
        );
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_channel_id_past_the_range_folds_into_it(
        admin: &signer, alice: &signer
    ) {
        setup(admin);
        // The generator hands out one channel per account, but nothing stops a
        // replay from naming one that was never opened.
        relay_endpoint::bench_onboard(alice, N_CHANNELS + 1, ONBOARD_PREPAY);
        relay_endpoint::bench_send(alice, N_CHANNELS + 1, payloads());
        assert!(relay_channel::outbound(1) == MSGS_PER_TXN, 0);
        assert!(relay_channel::outbound(0) == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_mix_runs_before_any_channel_is_open(
        admin: &signer, alice: &signer
    ) {
        // Only the top-level resources exist: no channel has been opened, so
        // every branch has to find nothing and return.
        relay_endpoint::initialize(admin, N_CHANNELS, SRC_EID, DST_EID);
        relay_endpoint::bench_onboard(alice, 0, ONBOARD_PREPAY);
        relay_endpoint::bench_send(alice, 0, payloads());
        relay_endpoint::bench_deliver(alice, 0, payloads());
        relay_endpoint::bench_verify(alice, 0, MSGS_PER_TXN, VERIFIERS_PER_MSG);
        relay_endpoint::bench_read_state(alice, 0, READ_DEPTH);
        relay_endpoint::bench_skip(alice, 0, 1);
        relay_endpoint::bench_set_config(alice, 0, VERIFIERS_PER_MSG, 1, 1);
        assert!(relay_endpoint::outbound_nonce(0) == 0, 0);
        assert!(relay_registry::n_oapps() == 0, 0);
    }
}
