/// Batch distribution, shaped after the mainnet campaign packages that pay a
/// list of addresses in one transaction: AniAirdrop's `airdrop` and the Amnis
/// campaign distributor.
///
/// Every entry point here takes a recipient list and fans out over it, so the
/// transaction cost is the storage cost of `N` slots and almost nothing else.
/// None of them can abort: a recipient with no slot gets one, a distributor
/// that is short on balance faucets the difference, and a repeated claim pays
/// zero rather than failing.
module bench::ad_distributor {
    use std::signer;
    use std::vector;
    use bench::ad_assets;
    use bench::ad_ledger;
    use bench::ad_registry;

    /// Only the package address can configure the campaign.
    const E_NOT_BENCH: u64 = 1;

    /// Symbol of the airdropped asset, created by the publisher before
    /// `initialize` runs.
    const SYMBOL: vector<u8> = b"ADROP";

    /// Widest fan-out a single transaction will do. Together with
    /// `MAX_UNIT_AMOUNT` this keeps the batch total inside a `u64`, so a
    /// caller cannot overflow the benchmark into an abort.
    const MAX_RECIPIENTS: u64 = 256;
    const MAX_UNIT_AMOUNT: u64 = 1_000_000_000;

    /// Faucetted on top of the shortfall, so a distributor tops up once in a
    /// while instead of on every batch.
    const REFILL: u64 = 1_000_000_000;

    fun min(a: u64, b: u64): u64 {
        if (a < b) a else b
    }

    /// Recipients of `batch` this package will actually walk.
    fun batch_len(batch: &vector<address>): u64 {
        min(vector::length(batch), MAX_RECIPIENTS)
    }

    /// Create the shards and the ledger. The asset is created separately so
    /// that the publisher picks its symbol and decimals.
    public entry fun initialize(admin: &signer, n_shards: u64) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        ad_registry::initialize(admin, n_shards);
        ad_ledger::initialize(admin);
    }

    /// Give every address in `recipients` a slot in `shard_id` before the mix
    /// starts, so the update branch has warm slots to modify. A whole shard at
    /// once runs past the per-transaction execution limit, so the caller
    /// chunks the list.
    public entry fun seed_recipients(
        admin: &signer, shard_id: u64, recipients: vector<address>, amount: u64
    ) {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let memo = vector::empty<u8>();
        credit_all(
            shard_id, &recipients, min(amount, MAX_UNIT_AMOUNT), &memo, false);
    }

    /// Bring a fresh account to the state the mix assumes: funded with the
    /// asset and holding a slot of its own, so the claim branch has something
    /// to pay out.
    public entry fun bench_onboard(
        user: &signer, shard_id: u64, fund_amount: u64, seed_amount: u64
    ) {
        let user_addr = signer::address_of(user);
        ad_assets::faucet(ad_assets::asset(@bench, SYMBOL), user_addr, fund_amount);
        ad_registry::credit(
            shard_id,
            user_addr,
            min(seed_amount, MAX_UNIT_AMOUNT),
            vector::empty<u8>(),
            false,
        );
    }

    /// Credit every recipient in the batch. Whether this writes new slots or
    /// modifies existing ones is decided entirely by the addresses the caller
    /// passes, so one entry point serves both distribution branches of the
    /// mix.
    ///
    /// The slots it creates are evictable, and the trim that follows takes the
    /// shard back down to `ring_cap` of them. A batch of addresses nothing has
    /// paid before therefore costs its creations plus the same number of
    /// deletions, which is what stops the run from growing the claims table
    /// under itself.
    public entry fun bench_distribute(
        _user: &signer,
        shard_id: u64,
        recipients: vector<address>,
        amount: u64,
        memo: vector<u8>,
        ring_cap: u64,
    ) {
        let amount = min(amount, MAX_UNIT_AMOUNT);
        let n = batch_len(&recipients);
        let fresh = credit_all(shard_id, &recipients, amount, &memo, true);
        ad_registry::trim(shard_id, ring_cap);
        ad_ledger::record_batch(
            shard_id,
            ad_ledger::kind_distribute(),
            n,
            fresh,
            amount * n,
        );
    }

    /// Pay the batch in the fungible asset itself, which puts a balance in
    /// each recipient's primary store at a derived address.
    public entry fun bench_fa_fanout(
        user: &signer, shard_id: u64, recipients: vector<address>, amount: u64
    ) {
        let amount = min(amount, MAX_UNIT_AMOUNT);
        let user_addr = signer::address_of(user);
        let asset = ad_assets::asset(@bench, SYMBOL);
        let n = batch_len(&recipients);
        let needed = amount * n;
        let balance = ad_assets::primary_balance(user_addr, asset);
        if (balance < needed) {
            ad_assets::faucet(asset, user_addr, needed - balance + REFILL);
        };
        let i = 0;
        while (i < n) {
            ad_assets::transfer_primary(
                user,
                asset,
                *vector::borrow(&recipients, i),
                amount,
            );
            i = i + 1;
        };
        ad_ledger::record_batch(
            shard_id,
            ad_ledger::kind_fa_fanout(),
            n,
            0,
            needed,
        );
    }

    /// Borrow every slot in the batch for writing but change only every
    /// `write_every`-th one. The distance between the slots a VM reports as
    /// written and the `written` this records is the over-approximation this
    /// branch exists to measure, so a stride position whose slot does not
    /// exist yet must not be counted: nothing was written there.
    public entry fun bench_touch(
        _user: &signer, shard_id: u64, recipients: vector<address>, write_every: u64
    ) {
        // A zero stride would make the modulo abort, so it means "write all".
        let stride = if (write_every == 0) 1 else write_every;
        let n = batch_len(&recipients);
        let written = 0;
        let i = 0;
        while (i < n) {
            let write = (i + 1) % stride == 0;
            if (ad_registry::touch(
                shard_id, *vector::borrow(&recipients, i), write)) {
                written = written + 1;
            };
            i = i + 1;
        };
        ad_ledger::record_batch(shard_id, ad_ledger::kind_touch(), n, written, 0);
    }

    /// Pay the caller whatever its own slot owes. A repeat claim finds zero
    /// and only bumps the counter.
    public entry fun bench_claim(user: &signer, shard_id: u64) {
        let user_addr = signer::address_of(user);
        let amount = ad_registry::claim(shard_id, user_addr);
        if (amount > 0) {
            ad_assets::faucet(ad_assets::asset(@bench, SYMBOL), user_addr, amount);
        };
        ad_ledger::record_claim(shard_id, amount);
    }

    /// Scan the batch without writing anything, so a comparison against the
    /// distribution branches separates read fan-out from write fan-out.
    public entry fun bench_sweep(
        _user: &signer, shard_id: u64, recipients: vector<address>
    ) {
        sweep(shard_id, &recipients);
    }

    /// Credit the batch and report how many slots it created. `track` decides
    /// whether those slots can later be evicted.
    fun credit_all(
        shard_id: u64,
        recipients: &vector<address>,
        amount: u64,
        memo: &vector<u8>,
        track: bool,
    ): u64 {
        let n = batch_len(recipients);
        let fresh = 0;
        let i = 0;
        while (i < n) {
            if (ad_registry::credit(
                shard_id, *vector::borrow(recipients, i), amount, *memo, track)) {
                fresh = fresh + 1;
            };
            i = i + 1;
        };
        fresh
    }

    /// Total owed across the batch. Absent slots read as zero.
    public fun sweep(shard_id: u64, recipients: &vector<address>): u64 {
        let n = batch_len(recipients);
        let total = 0;
        let i = 0;
        while (i < n) {
            total = total
                + ad_registry::peek(shard_id, *vector::borrow(recipients, i));
            i = i + 1;
        };
        total
    }

    public fun asset_symbol(): vector<u8> { SYMBOL }

    public fun max_recipients(): u64 { MAX_RECIPIENTS }

    #[view]
    public fun balance_of(owner: address): u64 {
        ad_assets::primary_balance(owner, ad_assets::asset(@bench, SYMBOL))
    }
}
