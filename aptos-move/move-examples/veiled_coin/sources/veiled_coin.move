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
    use aptos_std::elgamal::{Self, Ciphertext, CompressedCiphertext, PubKey};
    use aptos_std::ristretto255::{Self, Scalar, new_scalar_from_u64};

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::bulletproofs::RangeProof;
    use aptos_std::bulletproofs;

    //
    // Errors.
    //

    /// The range proof system needs to support proofs for any number \in [0, 2^{32})
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE : u64 = 1;

    /// A range proof failed to verify.
    const ERANGE_PROOF_VERIFICATION_FAILED : u64 = 2;

    /// Account already has `VeiledCoinStore` registered for `CoinType`
    const EVEILED_COIN_STORE_ALREADY_PUBLISHED: u64 = 4;

    /// Account hasn't registered `CoinStore` for `CoinType`
    const EVEILED_COIN_STORE_NOT_PUBLISHED: u64 = 5;

    /// Not enough coins to complete transaction
    const EINSUFFICIENT_BALANCE: u64 = 6;

    /// Byte vector failed to deserialize to ciphertexts
    const EDESERIALIZATION_FAILED: u64 = 7;

    //
    // Constants
    //

    /// The maximum number of bits used to represent a coin's value.
    const MAX_BITS_IN_VALUE : u64 = 32;

    /// The domain separation tag (DST) used for the Bulletproofs prover.
    const VEILED_COIN_DST : vector<u8> = b"AptosVeiledCoinExample";

    //
    // Core data structures.
    //

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// ElGamal ciphertext of a number of coins v \in [0, 2^{32}), an invariant
        /// that is enforced throughout the code
        private_value: Ciphertext,
    }

    /// A holder of a specific coin types and associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A ElGamal ciphertext of a value v \in [0, 2^{32}), an invariant that is enforced throughout the code.
        private_balance: CompressedCiphertext,
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
    fun init_module(deployer: &signer) {
        assert!(bulletproofs::get_max_range_bits() >= MAX_BITS_IN_VALUE, ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE);

        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // transfer Coin<T>'s into its CoinStore<T> before minting a VeiledCoin<T>.
        let (_resource, signer_cap) = account::create_resource_account(deployer, vector::empty());

        move_to(deployer,
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

        deposit<CoinType>(recipient, vc)
    }

    /// Sends the specified private amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount. Also requires a sigma protocol to prove that 
    /// 'private_withdraw_amount' encrypts the same value using the same randomness as 'private_deposit_amount'. 
    /// This value is the amount being transferred. These two ciphertexts are required as we need to update 
    /// both the sender's and the recipient's balances, which use different public keys and so must be updated 
    /// with ciphertexts encrypted with their respective public keys. 
    public entry fun private_transfer_to<CoinType>(
	sender: &signer, 
	recipient: address, 
	withdraw_ct: vector<u8>, 
	deposit_ct: vector<u8>, 
	range_proof_updated_balance: vector<u8>, 
	range_proof_transferred_amount: vector<u8>) acquires VeiledCoinStore {

	let private_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
	assert!(std::option::is_some(&private_withdraw_amount), EDESERIALIZATION_FAILED);
	let private_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
	assert!(std::option::is_some(&private_deposit_amount), EDESERIALIZATION_FAILED);

	transfer_privately_to<CoinType>(
		sender,
		recipient,
		std::option::extract(&mut private_withdraw_amount),
		std::option::extract(&mut private_deposit_amount),
		&bulletproofs::range_proof_from_bytes(range_proof_updated_balance),
		&bulletproofs::range_proof_from_bytes(range_proof_transferred_amount),
	)
    }

    /// Returns the ciphertext of the balance of `owner` for the provided `CoinType`.
    public fun private_balance<CoinType>(owner: address): CompressedCiphertext acquires VeiledCoinStore {
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

    /// Returns the ciphertext of the value of `coin`.
    public fun private_value<CoinType>(coin: &VeiledCoin<CoinType>): &Ciphertext {
        &coin.private_value
    }

    /// Returns true if the balance at address `owner` equals `value`, which should be useful for auditability. Requires
    /// the ElGamal ciphertext randomness as an auxiliary input.
    public fun verify_opened_balance<CoinType>(owner: address, pubkey: &PubKey, value: u64, randomness: &Scalar): bool acquires VeiledCoinStore {
        // compute the expected committed balance
        let value = new_scalar_from_u64(value);
        let expected_ct = elgamal::new_ciphertext_with_basepoint(&value, randomness, pubkey);

        // get the actual committed balance
        let actual_ct = elgamal::decompress_ciphertext(&private_balance<CoinType>(owner));

        elgamal::ciphertext_equals(&actual_ct, &expected_ct)
    }

    /// Initializes a veiled coin store for the specified account.
    public fun register<CoinType>(account: &signer) {
        let account_addr = signer::address_of(account);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED),
        );

        let coin_store = VeiledCoinStore<CoinType> {
            private_balance: elgamal::new_ciphertext_from_compressed(ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed()),
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
            private_value: elgamal::new_ciphertext_no_randomness(&value)
        }
    }

    /// Sends the specified private amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount. Also requires a sigma protocol to prove that 
    /// 'private_withdraw_amount' encrypts the same value using the same randomness as 'private_deposit_amount'. 
    /// This value is the amount being transferred. These two ciphertexts are required as we need to update 
    /// both the sender's and the recipient's balances, which use different public keys and so must be updated 
    /// with ciphertexts encrypted with their respective public keys. 
    public fun transfer_privately_to<CoinType>(
        sender: &signer,
        recipient: address,
        private_withdraw_amount: Ciphertext,
	private_deposit_amount: Ciphertext,
        range_proof_updated_balance: &RangeProof,
	range_proof_transferred_amount: &RangeProof)
    acquires VeiledCoinStore {
	// TODO: Insert sigma protocol here which proves 'private_deposit_amount' and 'private_withdraw_amount' encrypt the same values using the same randomness
        withdraw<CoinType>(sender, private_withdraw_amount, range_proof_updated_balance, range_proof_transferred_amount);
	let vc = VeiledCoin<CoinType> { private_value: private_deposit_amount };

        deposit(recipient, vc);
    }

    /// Sends the specified public amount to `recipient` and updates the private balance of `sender`. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a range proof on the transferred amount. 
    // TODO: Work out weird move-isms here
    /*public fun transfer_publicly_to<CoinType>(
        sender: &signer,
        recipient: address,
        public_amount: u64,
        range_proof_updated_balance: &RangeProof,
	range_proof_transferred_amount: &RangeProof)
    acquires VeiledCoinStore {
        let no_rand_ct = elgamal::new_ciphertext_no_randomness(&new_scalar_from_u64(public_amount));

        withdraw<CoinType>(sender, no_rand_ct, range_proof_updated_balance, range_proof_transferred_amount);

	let vc = VeiledCoin<CoinType> { private_value: no_rand_ct };

        deposit(recipient, vc);
    }*/

    /// Deposits a veiled coin at address `to_addr`.
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

        let private_balance = elgamal::decompress_ciphertext(&coin_store.private_balance);

        elgamal::ciphertext_add_assign(&mut private_balance, &coin.private_value);

        coin_store.private_balance = elgamal::compress_ciphertext(&private_balance);

        drop_veiled_coin(coin);
    }

    /// Withdraws the specifed private `amount` of veiled coin `CoinType` from the signing account.
    public fun withdraw<CoinType>(
        account: &signer,
        withdraw_amount: Ciphertext,
        range_proof_updated_balance: &RangeProof,
	range_proof_transferred_amount: &RangeProof,
    ) acquires VeiledCoinStore {
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
        let private_balance = elgamal::decompress_ciphertext(&coin_store.private_balance);

        elgamal::ciphertext_sub_assign(&mut private_balance, &withdraw_amount);

        // This function is splitting a commitment 'bal' into a commitment 'amount' and a commitment 'new_bal' =
        // = 'bal' - 'amount'. We assume that 'bal' is in [0, 2^{32}). To prevent underflow, we need to
        // verify a range proof that 'new_bal' is in [0, 2^{32}). In addition we need to verify a range proof 
	// that the transferred amount 'amount' is in [0, 2^{32}). Otherwise, a sender could send 'amount' = p-1
	// where p is the order of the scalar field, giving an updated balance of 
	// 'bal' - (p-1) mod p = 'bal' + 1. These checks ensure that 'bal' - 'amount' >= 0 
	// and therefore that 'bal' >= 'amount'.
        assert!(bulletproofs::verify_range_proof(&private_balance, range_proof_updated_balance, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);
	assert!(bulletproofs::verify_range_proof(&withdraw_amount, range_proof_transferred_amount, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);

        coin_store.private_balance = elgamal::compress_ciphertext(&private_balance);

        // Returns the withdrawn veiled coin
        //VeiledCoin<CoinType> {
        //    private_value
        //}
    }

    //
    // Tests
    //

    const SOME_RANDOMNESS_1: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

    #[test(myself = @veiled_coin, source_fx = @aptos_framework, destination = @0x1337)]
    fun basic_viability_test(
        myself: signer,
        source_fx: signer,
        destination: signer
    ) acquires VeiledCoinMinter, VeiledCoinStore {
        // Initialize the `veiled_coin` module
        init_module(&myself);

        features::change_feature_flags(&source_fx, vector[features::get_bulletproofs_feature()], vector[]);

        // Set up two accounts so we can register a new coin type on them
        let source_addr = signer::address_of(&source_fx);
        account::create_account_for_test(source_addr);
        let destination_addr = signer::address_of(&destination);
        account::create_account_for_test(destination_addr);

        // Create some 1,000 fake money inside 'source'
        coin::create_fake_money(&source_fx, &destination, 1000);

        // Split 500 and 500 between source and destination
        coin::transfer<coin::FakeMoney>(&source_fx, destination_addr, 500);

        // Mint 100 veiled coins at source (requires registering a veiled coin store at 'source')
        register<coin::FakeMoney>(&source_fx);
        mint<coin::FakeMoney>(&source_fx, 100);

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
        transfer_privately_to<coin::FakeMoney>(&source_fx, destination_addr, pedersen::commitment_clone(&comm), &range_proof);

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
