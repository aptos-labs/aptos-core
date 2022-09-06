/// This module provides a veiled coin type, denoted `VeiledCoin<T>` that hides the value/denomination of a coin.
///
/// Our implementation relies on a secondary so-called "resource account" which helps us mint a `VeiledCoin<T>` from a
/// traditional `coin::Coin<T>` by transferring this latter coin into a `coin::CoinStore<T>` resource stored in the
/// resource account. Later on, when someone wants to convert their `VeiledCoin<T>` into a traditional `coin::Coin<T>`
/// the resource account can be used to transfer out said `coin::Coin<T>` from its coin store. This is where the
/// "resource account" becomes important, since transfering out a coin like this requires a `signer` for the resource
/// account, which this module can obtain via a `SignerCapability`.
module veiled_coin::veiled_coin {
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_std::event::{Self, EventHandle};
    use aptos_std::pedersen::{Self, Commitment};
    use aptos_std::ristretto255::{Self, CompressedRistretto, Scalar, new_scalar_from_u64, new_scalar_from_bytes, scalar_neg, point_equals, point_decompress};

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_std::bulletproofs::RangeProof;
    use aptos_std::bulletproofs;

    //
    // Errors.
    //

    /// The range proof system needs to support proofs for any number \in [0, 2^{64})
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE : u64 = 1;

    /// A range proof failed to verify.
    const ERANGE_PROOF_VERIFICATION_FAILED : u64 = 2;

    /// Account already has `VeiledCoinStore` registered for `CoinType`
    const EVEILED_COIN_STORE_ALREADY_PUBLISHED: u64 = 4;

    /// Account hasn't registered `CoinStore` for `CoinType`
    const EVEILED_COIN_STORE_NOT_PUBLISHED: u64 = 5;

    /// Not enough coins to complete transaction
    const EINSUFFICIENT_BALANCE: u64 = 6;

    //
    // Constants
    //

    /// The maximum number of bits used to represent a coin's value.
    const MAX_BITS_IN_VALUE : u64 = 64;

    /// The domain separation tag (DST) used for the Bulletproofs prover.
    const VEILED_COIN_DST : vector<u8> = b"AptosVeiledCoinExample";

    //
    // Core data structures.
    //

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// Pedersen commitment (under the default Bulletproofs CK) to a number of coins v \in [0, 2^{64}), an invariant
        /// that is enforced throughout the code
        private_value: Commitment,
    }

    /// A holder of a specific coin types and associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A Pedersen commitment to a value v \in [0, 2^{64}), an invariant that is enforced throughout the code.
        private_balance: CompressedRistretto,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
    }

    /// Holds a signer capability for the resource account created when initializing this module. This account houses a
    /// coin::CoinStore<T> for every type of coin T that is veiled.
    struct VeiledCoinMinter has store, key {
        signer_cap: account::SignerCapability,
    }

    /// Event emitted when some amount of a coin is deposited into an account.
    struct DepositEvent has drop, store {
        // We cannot leak any information about how much has been deposited.
    }

    /// Event emitted when some amount of a coin is withdrawn from an account.
    struct WithdrawEvent has drop, store {
        // We cannot leak any information about how much has been withdrawn.
    }

    //
    // Module initialization, done only once when this module is first published on the blockchain
    //

    /// Initializes a so-called "resource" account which will maintain a coin::CoinStore<T> resource for all Coin<T>'s
    /// that have been converted into a VeiledCoin<T>.
    fun init_module(sender: &signer) {
        assert!(bulletproofs::get_max_range_bits() >= 64, ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE);

        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // transfer Coin<T>'s into its CoinStore<T> before minting a VeiledCoin<T>.
        let (_resource, signer_cap) = account::create_resource_account(sender, vector::empty());

        move_to(sender,
            VeiledCoinMinter {
                signer_cap
            }
        )
    }

    //
    // Entry functions
    //

    /// Takes `amount` of `coin::Coin<CoinType>` coins from `sender`, wraps them inside a `VeiledCoin<CoinType>` and
    /// sends them back to `sender`.
    public entry fun mint<CoinType>(sender: &signer, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        mint_to<CoinType>(sender, signer::address_of(sender), amount)
    }

    /// Takes `amount` of `coin::Coin<CoinType>` coins from `sender`, wraps them inside a `VeiledCoin<CoinType>` and\
    /// sends to `recipient`.
    public entry fun mint_to<CoinType>(sender: &signer, recipient: address, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        let c = coin::withdraw<CoinType>(sender, amount);

        let vc = mint_from_coin(c);

        deposit(recipient, vc)
    }

    /// Returns the commitment to the balance of `owner` for the provided `CoinType`.
    public entry fun private_balance<CoinType>(owner: address): CompressedRistretto acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(owner),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        borrow_global<VeiledCoinStore<CoinType>>(owner).private_balance
    }

    //
    // Private functions.
    //

    /// Returns a signer for the resource account.
    fun get_resource_account_signer(): signer acquires VeiledCoinMinter {
        account::create_signer_with_capability(&borrow_global<VeiledCoinMinter>(@veiled_coin).signer_cap)
    }

    /// Used internally to drop veiled coins that were split or joined.
    fun drop_veiled_coin<CoinType>(c: VeiledCoin<CoinType>) {
        let VeiledCoin<CoinType> { private_value: _ } = c;
    }

    //
    // Public functions.
    //

    /// Returns `true` if `account_addr` is registered to receive veiled coins of `CoinType`.
    public fun has_veiled_coin_store<CoinType>(account_addr: address): bool {
        exists<VeiledCoinStore<CoinType>>(account_addr)
    }

    /// Returns the commitment to the value of `coin`.
    public fun private_value<CoinType>(coin: &VeiledCoin<CoinType>): &Commitment {
        &coin.private_value
    }

    /// Returns true if the balance at address `owner` equals `value`, which should be useful for auditability. Requires
    /// the Pedersen commitment randomness as an auxiliary input.
    public fun verify_opened_balance<CoinType>(owner: address, value: u64, randomness: &Scalar): bool acquires VeiledCoinStore {
        // compute the expected committed balance
        let value = new_scalar_from_u64(value);
        let expected_comm = pedersen::new_commitment_for_bulletproof(&value, randomness);

        // get the actual committed balance
        let actual_comm = pedersen::new_commitment_from_compressed(&private_balance<CoinType>(owner));

        pedersen::commitment_equals(&actual_comm, &expected_comm)
    }

    /// Initializes a veiled coin store for the specified account.
    public fun register<CoinType>(account: &signer) {
        let account_addr = signer::address_of(account);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED),
        );

        let coin_store = VeiledCoinStore<CoinType> {
            private_balance: ristretto255::point_identity_compressed(),
            deposit_events: account::new_event_handle<DepositEvent>(account),
            withdraw_events: account::new_event_handle<WithdrawEvent>(account),
        };
        move_to(account, coin_store);
    }

    /// Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.
    ///
    /// WARNING: Fundamentally, there is no way to hide the value of the coin being minted here.
    public fun mint_from_coin<CoinType>(c: Coin<CoinType>): VeiledCoin<CoinType> acquires VeiledCoinMinter {
        // If there is no CoinStore<CoinType> yet, create one.
        let rsrc_acc_signer = get_resource_account_signer();
        let rsrc_acc_addr = signer::address_of(&rsrc_acc_signer);
        if(!coin::is_account_registered<CoinType>(rsrc_acc_addr)) {
            coin::register<CoinType>(&rsrc_acc_signer);
        };

        // Move the traditional coin into the coin store, so we can mint a veiled coin.
        // (There is no other way to drop a traditional coin, for safety reasons, so moving it into a coin store is
        //  the only option.)
        let value = new_scalar_from_u64(coin::value(&c));
        coin::deposit(rsrc_acc_addr, c);

        VeiledCoin<CoinType> {
            private_value: pedersen::new_non_hiding_commitment_for_bulletproof(&value)
        }
    }

    /// Sends the specified private amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send. `private_amount` is a
    /// Pedersen commitment to the amount being sent.
    public fun transfer_privately_to<CoinType>(
        sender: &signer,
        recipient: address,
        private_amount: Commitment,
        range_proof: &RangeProof)
    acquires VeiledCoinStore {
        let vc = withdraw<CoinType>(sender, private_amount, range_proof);

        deposit(recipient, vc);
    }

    /// Sends the specified public amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send.
    public fun transfer_publicly_to<CoinType>(
        sender: &signer,
        recipient: address,
        public_amount: u64,
        range_proof: &RangeProof)
    acquires VeiledCoinStore {
        let non_hiding_comm = pedersen::new_non_hiding_commitment_for_bulletproof(&new_scalar_from_u64(public_amount));

        let vc = withdraw<CoinType>(sender, non_hiding_comm, range_proof);

        deposit(recipient, vc);
    }

    /// Deposits a veiled coin at address `to_addr`.
    ///
    /// WARNING: Assumes the owner of `to_addr` somehow obtains the randomness of `coin` out-of-band, so they can
    /// later spend (part of) their private balance.
    public fun deposit<CoinType>(to_addr: address, coin: VeiledCoin<CoinType>) acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(to_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(to_addr);

        event::emit_event<DepositEvent>(
            &mut coin_store.deposit_events,
            DepositEvent {},
        );

        let private_balance = pedersen::new_commitment_from_compressed(&coin_store.private_balance);

        pedersen::commitment_add_assign(&mut private_balance, &coin.private_value);

        coin_store.private_balance = pedersen::commitment_as_compressed_point(&private_balance);

        drop_veiled_coin(coin);
    }

    /// Withdraws the specifed private `amount` of veiled coin `CoinType` from the signing account.
    public fun withdraw<CoinType>(
        account: &signer,
        private_value: Commitment,
        range_proof: &RangeProof,
    ): VeiledCoin<CoinType> acquires VeiledCoinStore {
        let account_addr = signer::address_of(account);
        assert!(
            has_veiled_coin_store<CoinType>(account_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(account_addr);

        event::emit_event<WithdrawEvent>(
            &mut coin_store.withdraw_events,
            WithdrawEvent { },
        );

        // Update the coin store by homomorphically subtracting the committed withdrawn amount from the committed balance.
        let private_balance = pedersen::new_commitment_from_compressed(&coin_store.private_balance);

        pedersen::commitment_sub_assign(&mut private_balance, &private_value);

        // This function is splitting a commitment 'bal' into a commitment 'amount' and a commitment 'new_bal' =
        // = 'bal' - 'amount'. The key invariant we want to enforce is that 'bal' is in range [0, 2^{64}). Therefore, we verify a
        // proof that 'new_bal' is in range. This implies that 'bal' - 'amount' >= 0 and therefore that 'bal' >= 'amount'.
        assert!(bulletproofs::verify_range_proof(&private_balance, range_proof, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);

        coin_store.private_balance = pedersen::commitment_as_compressed_point(&private_balance);

        // Returns the withdrawn veiled coin
        VeiledCoin<CoinType> {
            private_value
        }
    }

    //
    // Tests
    //

    const SOME_RANDOMNESS_1: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

    #[test(myself = @veiled_coin, source = @0x1, destination = @0x2)]
    fun basic_viability_test(
        myself: signer,
        source: signer,
        destination: signer
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        // Initialzie the veiled coin module
        init_module(&myself);

        // Set up two accounts so we can register a new coin type on them
        let source_addr = signer::address_of(&source);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        // Create some 1,000 fake money inside 'source'
        coin::create_fake_money(&source, &destination, 1000);

        // Split 500 and 500 between source and destination
        coin::transfer<coin::FakeMoney>(&source, destination_addr, 500);

        // Mint 100 veiled coins at source (requires registering a veiled coin store at 'source')
        register<coin::FakeMoney>(&source);
        mint<coin::FakeMoney>(&source, 100);

        // Transfer 50 of these veiled coins to destination
        let val = new_scalar_from_u64(50);
        let rand = new_scalar_from_bytes(SOME_RANDOMNESS_1);
        let rand = std::option::extract(&mut rand);

        let comm = pedersen::new_commitment_for_bulletproof(&val, &rand);

        // This will be the balance left at the source, that we need to do a range proof for
        let source_randomness = scalar_neg(&rand);
        let source_new_val = new_scalar_from_u64(50);
        let (range_proof, source_new_comm) = bulletproofs::prove_range(&source_new_val, &source_randomness, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        // Execute the veiled transaction: no one will be able to tell 50 coins are being transferred.
        register<coin::FakeMoney>(&destination);
        transfer_privately_to<coin::FakeMoney>(&source, destination_addr, pedersen::commitment_clone(&comm), &range_proof);

        // Sanity check veiled balances
        assert!(verify_opened_balance<coin::FakeMoney>(source_addr, 50, &source_randomness), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(destination_addr, 50, &rand), 1);

        assert!(point_equals(
            pedersen::commitment_as_point(&comm),
            &point_decompress(&private_balance<coin::FakeMoney>(destination_addr))
        ), 1);

        assert!(point_equals(
            pedersen::commitment_as_point(&source_new_comm),
            &point_decompress(&private_balance<coin::FakeMoney>(source_addr))
        ), 1);
    }
}
