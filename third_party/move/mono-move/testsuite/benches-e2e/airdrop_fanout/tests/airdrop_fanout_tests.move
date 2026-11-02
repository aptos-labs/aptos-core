#[test_only]
module bench::airdrop_fanout_tests {
    use std::bcs;
    use std::hash;
    use std::signer;
    use std::vector;
    use aptos_std::from_bcs;
    use bench::ad_assets;
    use bench::ad_distributor;
    use bench::ad_ledger;
    use bench::ad_registry;

    const SYMBOL: vector<u8> = b"ADROP";
    const DECIMALS: u8 = 8;

    /// Recipients per `seed_recipients` call, matching the generator.
    const SEED_CHUNK: u64 = 64;

    /// The generator's knobs, scaled down to fit the unit-test gas bound. The
    /// shape is what these tests pin, so only the counts shrink: seeding still
    /// spans more than one chunk and a touch batch is still a whole multiple
    /// of the stride.
    const TEST_SHARDS: u64 = 2;
    const TEST_WARM: u64 = 128;
    const TEST_BATCH: u64 = 16;
    const PAYLOAD_LEN: u64 = 32;
    const WRITE_EVERY: u64 = 4;
    const FRESH_RATIO: u64 = 75;

    /// Evictable slots a shard keeps. A whole multiple of what one batch
    /// creates, so the equilibrium sits exactly on it, and wide enough that
    /// the tests which are not about eviction never reach it.
    const TEST_RING_CAP: u64 = 24;

    const SEED_AMOUNT: u64 = 1000000;
    const DISTRIBUTE_AMOUNT: u64 = 1000;
    const ONBOARD_FUNDING: u64 = 1000000000000;

    fun min(a: u64, b: u64): u64 {
        if (a < b) a else b
    }

    fun append_be_u64(bytes: &mut vector<u8>, value: u64) {
        let i: u8 = 8;
        while (i > 0) {
            i = i - 1;
            vector::push_back(bytes, (((value >> (i * 8)) & 0xFF) as u8));
        }
    }

    /// Mirrors the generator's `slot_for`: the low eight address bytes read as
    /// a little-endian `u64`, taken modulo the shard count.
    fun slot_for(account: address, modulus: u64): u64 {
        let bytes = bcs::to_bytes(&account);
        let tail = 0;
        let i: u8 = 8;
        while (i > 0) {
            i = i - 1;
            tail = (tail << 8)
                | (*vector::borrow(&bytes, 24 + (i as u64)) as u64);
        };
        tail % modulus
    }

    /// Mirrors the generator's warm recipient derivation, so two accounts on
    /// one shard land on the same slots.
    fun warm_recipient(shard: u64, index: u64): address {
        let bytes = vector::empty<u8>();
        let i = 0;
        while (i < 16) {
            vector::push_back(&mut bytes, 0);
            i = i + 1;
        };
        append_be_u64(&mut bytes, shard + 1);
        append_be_u64(&mut bytes, index);
        from_bcs::to_address(bytes)
    }

    /// Mirrors the generator's fresh recipient derivation.
    fun fresh_recipient(account: address, counter: u64): address {
        let input = bcs::to_bytes(&account);
        vector::append(&mut input, bcs::to_bytes(&counter));
        from_bcs::to_address(hash::sha3_256(input))
    }

    fun warm_batch(shard: u64, start: u64, n: u64): vector<address> {
        let batch = vector::empty<address>();
        let i = 0;
        while (i < n) {
            vector::push_back(
                &mut batch, warm_recipient(shard, (start + i) % TEST_WARM));
            i = i + 1;
        };
        batch
    }

    fun fresh_batch(
        account: address, shard: u64, start: u64, counter: u64, n: u64
    ): vector<address> {
        let fresh = n * FRESH_RATIO / 100;
        let batch = vector::empty<address>();
        let i = 0;
        while (i < n) {
            let recipient =
                if (i < fresh) {
                    fresh_recipient(account, counter + i)
                } else {
                    warm_recipient(shard, (start + i) % TEST_WARM)
                };
            vector::push_back(&mut batch, recipient);
            i = i + 1;
        };
        batch
    }

    /// Mirrors the generator's `with_own_slot`: a distribute batch pays its
    /// own sender in the last position, which is the only thing in the mix
    /// that credits an address a benchmark account can sign for.
    fun distribute_batch(sender: address, batch: vector<address>): vector<address> {
        let n = vector::length(&batch);
        if (n > 0) {
            *vector::borrow_mut(&mut batch, n - 1) = sender;
        };
        batch
    }

    fun memo_bytes(): vector<u8> {
        let memo = vector::empty<u8>();
        let i = 0;
        while (i < PAYLOAD_LEN) {
            vector::push_back(&mut memo, 0xAD);
            i = i + 1;
        };
        memo
    }

    /// The publisher-signed calls that come before any seeding.
    fun configure(admin: &signer) {
        ad_assets::create_asset_entry(admin, SYMBOL, DECIMALS);
        ad_distributor::initialize(admin, TEST_SHARDS);
    }

    fun seed_all(admin: &signer) {
        let shard = 0;
        while (shard < TEST_SHARDS) {
            let start = 0;
            while (start < TEST_WARM) {
                ad_distributor::seed_recipients(
                    admin,
                    shard,
                    warm_batch(shard, start, min(SEED_CHUNK, TEST_WARM - start)),
                    SEED_AMOUNT,
                );
                start = start + SEED_CHUNK;
            };
            shard = shard + 1;
        }
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        configure(admin);
        seed_all(admin);
        assert!(ad_registry::n_shards() == TEST_SHARDS, 0);
        assert!(ad_registry::shard_slots(0) == TEST_WARM, 0);
        assert!(ad_registry::shard_slots(1) == TEST_WARM, 0);
        assert!(ad_registry::has_slot(0, warm_recipient(0, 0)), 0);
        assert!(ad_registry::has_slot(1, warm_recipient(1, TEST_WARM - 1)), 0);

        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let shard = slot_for(alice_addr, TEST_SHARDS);
        ad_distributor::bench_onboard(
            alice, shard, ONBOARD_FUNDING, SEED_AMOUNT);
        // The two addresses hash to one shard, which is what the second half
        // of this test needs.
        assert!(slot_for(bob_addr, TEST_SHARDS) == shard, 0);
        ad_distributor::bench_onboard(
            bob, shard, ONBOARD_FUNDING, SEED_AMOUNT);
        assert!(ad_distributor::balance_of(alice_addr) == ONBOARD_FUNDING, 0);
        assert!(ad_registry::has_slot(shard, alice_addr), 0);

        let warm = warm_batch(shard, 0, TEST_BATCH);
        let warm_dist = distribute_batch(alice_addr, warm);
        let fresh = distribute_batch(
            alice_addr, fresh_batch(alice_addr, shard, 0, 7, TEST_BATCH));
        let memo = memo_bytes();
        let batches = ad_ledger::batches();
        let slots = ad_registry::shard_slots(shard);

        ad_distributor::bench_distribute(
            alice, shard, warm_dist, DISTRIBUTE_AMOUNT, memo, TEST_RING_CAP);
        // A warm batch modifies seeded slots rather than creating any.
        assert!(ad_registry::shard_slots(shard) == slots, 0);
        assert!(
            ad_registry::memo_len(shard, warm_recipient(shard, 0))
                == PAYLOAD_LEN,
            0,
        );

        ad_distributor::bench_distribute(
            alice, shard, fresh, DISTRIBUTE_AMOUNT, memo, TEST_RING_CAP);
        let created = TEST_BATCH * FRESH_RATIO / 100;
        assert!(ad_registry::shard_slots(shard) == slots + created, 0);

        ad_distributor::bench_fa_fanout(alice, shard, warm, DISTRIBUTE_AMOUNT);
        assert!(
            ad_assets::primary_balance(
                warm_recipient(shard, 0), ad_assets::asset(@bench, SYMBOL))
                == DISTRIBUTE_AMOUNT,
            0,
        );

        ad_distributor::bench_touch(alice, shard, warm, WRITE_EVERY);

        // The two distributes above each paid the sender's own slot on top of
        // the onboarding credit, so the claim has something to pay out.
        let owed = SEED_AMOUNT + 2 * DISTRIBUTE_AMOUNT;
        let (amount, _, _, _) = ad_registry::slot_state(shard, alice_addr);
        assert!(amount == owed, 0);
        let before = ad_distributor::balance_of(alice_addr);
        ad_distributor::bench_claim(alice, shard);
        assert!(ad_distributor::balance_of(alice_addr) == before + owed, 0);
        assert!(ad_ledger::claims() == 1, 0);
        ad_distributor::bench_sweep(alice, shard, warm);

        // Every branch but the read-only sweep records a batch. Claims count
        // separately, so only the four fan-out branches add recipients.
        assert!(ad_ledger::batches() == batches + 5, 0);
        assert!(ad_ledger::recipients() == 4 * TEST_BATCH, 0);

        // Two accounts pointed at one shard land on the same slots, so the
        // second account's batch modifies rather than creates.
        let slots = ad_registry::shard_slots(shard);
        ad_distributor::bench_distribute(
            bob,
            shard,
            distribute_batch(bob_addr, warm),
            DISTRIBUTE_AMOUNT,
            memo,
            TEST_RING_CAP,
        );
        assert!(ad_registry::shard_slots(shard) == slots, 0);
        let (_, credits, _, _) =
            ad_registry::slot_state(shard, warm_recipient(shard, 0));
        assert!(credits == 3, 0);
    }

    // Constraint 5. A campaign never pays the same cohort twice, so the fresh
    // branch creates slots that nothing will ever credit again, and the run
    // would deepen the tree under itself for as long as it lasts. The trim
    // behind each distribute takes back what that branch creates, so the
    // claims table settles at the ring cap and stays there.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_fresh_distribution_settles_at_the_ring_cap(
        admin: &signer, alice: &signer
    ) {
        configure(admin);
        seed_all(admin);
        let alice_addr = signer::address_of(alice);
        let shard = slot_for(alice_addr, TEST_SHARDS);
        let per_batch = TEST_BATCH * FRESH_RATIO / 100;
        let rounds = 8;
        assert!(rounds * per_batch > TEST_RING_CAP, 0);

        let i = 0;
        while (i < rounds) {
            ad_distributor::bench_distribute(
                alice,
                shard,
                fresh_batch(alice_addr, shard, 0, i * TEST_BATCH, TEST_BATCH),
                DISTRIBUTE_AMOUNT,
                memo_bytes(),
                TEST_RING_CAP,
            );
            assert!(ad_registry::shard_tracked(shard) <= TEST_RING_CAP, 0);
            i = i + 1;
        };
        // Filled and held, however many more rounds run.
        assert!(ad_registry::shard_tracked(shard) == TEST_RING_CAP, 0);
        // The oldest cohort is gone from the table, and the warm recipients
        // the other branches draw from are untouched by the eviction.
        assert!(!ad_registry::has_slot(shard, fresh_recipient(alice_addr, 0)), 0);
        assert!(ad_registry::has_slot(shard, warm_recipient(shard, 0)), 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_distribute_tolerates_unseeded_recipient(
        admin: &signer, alice: &signer
    ) {
        configure(admin);
        let batch = vector::empty<address>();
        vector::push_back(&mut batch, @0xdead);
        assert!(!ad_registry::has_slot(0, @0xdead), 0);
        ad_distributor::bench_distribute(
            alice, 0, batch, DISTRIBUTE_AMOUNT, memo_bytes(), TEST_RING_CAP);
        let (amount, credits, _, _) = ad_registry::slot_state(0, @0xdead);
        assert!(amount == DISTRIBUTE_AMOUNT && credits == 1, 0);
        assert!(ad_ledger::fresh_slots() == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_claim_tolerates_repeat(admin: &signer, alice: &signer) {
        configure(admin);
        let alice_addr = signer::address_of(alice);
        ad_distributor::bench_onboard(alice, 0, 0, SEED_AMOUNT);
        ad_distributor::bench_claim(alice, 0);
        let paid = ad_distributor::balance_of(alice_addr);
        assert!(paid == SEED_AMOUNT, 0);
        ad_distributor::bench_claim(alice, 0);
        assert!(ad_distributor::balance_of(alice_addr) == paid, 0);
        let (amount, _, claims, _) = ad_registry::slot_state(0, alice_addr);
        assert!(amount == 0 && claims == 2, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_fa_fanout_tolerates_short_balance(
        admin: &signer, alice: &signer
    ) {
        configure(admin);
        // Onboarded with nothing, so the batch is worth more than the caller
        // holds and the distributor has to faucet the difference.
        ad_distributor::bench_onboard(alice, 0, 0, 0);
        assert!(ad_distributor::balance_of(signer::address_of(alice)) == 0, 0);
        let batch = warm_batch(0, 0, 4);
        ad_distributor::bench_fa_fanout(alice, 0, batch, SEED_AMOUNT);
        let asset = ad_assets::asset(@bench, SYMBOL);
        let i = 0;
        while (i < 4) {
            assert!(
                ad_assets::primary_balance(*vector::borrow(&batch, i), asset)
                    == SEED_AMOUNT,
                0,
            );
            i = i + 1;
        }
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_batch_tolerates_empty_recipients(admin: &signer, alice: &signer) {
        configure(admin);
        let batches = ad_ledger::batches();
        let empty = vector::empty<address>();
        ad_distributor::bench_distribute(
            alice, 0, empty, DISTRIBUTE_AMOUNT, memo_bytes(), TEST_RING_CAP);
        ad_distributor::bench_fa_fanout(alice, 0, empty, DISTRIBUTE_AMOUNT);
        ad_distributor::bench_touch(alice, 0, empty, WRITE_EVERY);
        ad_distributor::bench_sweep(alice, 0, empty);
        assert!(ad_ledger::batches() == batches + 3, 0);
        assert!(ad_ledger::recipients() == 0, 0);
        assert!(ad_registry::shard_slots(0) == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_touch_writes_exactly_n_over_write_every(
        admin: &signer, alice: &signer
    ) {
        configure(admin);
        let batch = warm_batch(0, 0, TEST_BATCH);
        ad_distributor::seed_recipients(admin, 0, batch, SEED_AMOUNT);
        let recorded = ad_ledger::fresh_slots();
        ad_distributor::bench_touch(alice, 0, batch, WRITE_EVERY);

        let changed = 0;
        let unchanged = 0;
        let i = 0;
        while (i < TEST_BATCH) {
            let (_, _, _, touches) =
                ad_registry::slot_state(0, *vector::borrow(&batch, i));
            if (touches == 0) {
                unchanged = unchanged + 1;
            } else {
                changed = changed + 1;
            };
            i = i + 1;
        };
        assert!(changed == TEST_BATCH / WRITE_EVERY, 0);
        assert!(unchanged == TEST_BATCH - TEST_BATCH / WRITE_EVERY, 0);
        // The recorded count is the ground truth a VM's reported write count
        // is compared against, so it has to be the slots that really changed.
        assert!(ad_ledger::fresh_slots() == recorded + changed, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_touch_does_not_count_absent_slots(admin: &signer, alice: &signer) {
        configure(admin);
        let batch = warm_batch(0, 0, TEST_BATCH);
        // Only the front half of the batch has slots, so only the stride
        // positions inside that half can be written.
        let seeded = TEST_BATCH / 2;
        ad_distributor::seed_recipients(
            admin, 0, warm_batch(0, 0, seeded), SEED_AMOUNT);
        let recorded = ad_ledger::fresh_slots();
        ad_distributor::bench_touch(alice, 0, batch, WRITE_EVERY);

        let changed = 0;
        let i = 0;
        while (i < TEST_BATCH) {
            let (_, _, _, touches) =
                ad_registry::slot_state(0, *vector::borrow(&batch, i));
            changed = changed + touches;
            i = i + 1;
        };
        assert!(changed == seeded / WRITE_EVERY, 0);
        assert!(ad_ledger::fresh_slots() == recorded + changed, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_touch_tolerates_zero_stride(admin: &signer, alice: &signer) {
        configure(admin);
        let batch = warm_batch(0, 0, 4);
        ad_distributor::seed_recipients(admin, 0, batch, SEED_AMOUNT);
        ad_distributor::bench_touch(alice, 0, batch, 0);
        let (_, _, _, touches) = ad_registry::slot_state(0, warm_recipient(0, 0));
        assert!(touches == 1, 0);
    }

    // A claim only does work if something has credited the caller's own slot,
    // and the distribute batches the generator sends are the only thing that
    // ever does. Point them somewhere else and this fails.
    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_distribute_credits_its_sender_so_claims_pay(
        admin: &signer, alice: &signer
    ) {
        configure(admin);
        let alice_addr = signer::address_of(alice);
        let shard = slot_for(alice_addr, TEST_SHARDS);
        ad_distributor::bench_onboard(alice, shard, 0, SEED_AMOUNT);

        // Drain the onboarding credit first, so whatever the second claim
        // finds can only have come from the distribute in between.
        ad_distributor::bench_claim(alice, shard);
        let (amount, _, _, _) = ad_registry::slot_state(shard, alice_addr);
        assert!(amount == 0, 0);

        ad_distributor::bench_distribute(
            alice,
            shard,
            distribute_batch(alice_addr, warm_batch(shard, 0, TEST_BATCH)),
            DISTRIBUTE_AMOUNT,
            memo_bytes(),
            TEST_RING_CAP,
        );
        let (amount, _, _, _) = ad_registry::slot_state(shard, alice_addr);
        assert!(amount == DISTRIBUTE_AMOUNT, 0);

        let before = ad_distributor::balance_of(alice_addr);
        ad_distributor::bench_claim(alice, shard);
        assert!(
            ad_distributor::balance_of(alice_addr)
                == before + DISTRIBUTE_AMOUNT,
            0,
        );
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_configure_tolerates_repeat(admin: &signer, alice: &signer) {
        configure(admin);
        configure(admin);
        assert!(ad_registry::n_shards() == TEST_SHARDS, 0);
        ad_distributor::bench_onboard(alice, 0, ONBOARD_FUNDING, SEED_AMOUNT);
        assert!(
            ad_distributor::balance_of(signer::address_of(alice))
                == ONBOARD_FUNDING,
            0,
        );
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_shard_id_past_the_end_wraps(admin: &signer, alice: &signer) {
        configure(admin);
        let batch = vector::empty<address>();
        vector::push_back(&mut batch, @0xbeef);
        ad_distributor::bench_distribute(
            alice,
            TEST_SHARDS + 1,
            batch,
            DISTRIBUTE_AMOUNT,
            memo_bytes(),
            TEST_RING_CAP,
        );
        assert!(ad_registry::has_slot(1, @0xbeef), 0);
    }
}
