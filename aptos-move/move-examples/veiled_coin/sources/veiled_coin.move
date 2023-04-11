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
    //use std::features;
    use std::vector;
    use std::option::Option;
    use aptos_std::elgamal::{Self, Ciphertext, CompressedCiphertext, Pubkey};
    use aptos_std::ristretto255::{RistrettoPoint, Self, Scalar, new_scalar_from_u64};

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

    /// Account hasn't registered `VeiledCoinStore` for `CoinType`
    const EVEILED_COIN_STORE_NOT_PUBLISHED: u64 = 5;

    /// Not enough coins to complete transaction
    const EINSUFFICIENT_BALANCE: u64 = 6;

    /// Byte vector failed to deserialize to ciphertexts
    const EDESERIALIZATION_FAILED: u64 = 7;

    /// Byte vector given for deserialization was the wrong length
    const EBYTES_WRONG_LENGTH: u64 = 8;

    /// Sigma protocol for withdrawals failed
    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 9;

    /// Ciphertext has wrong value when unwrapping
    const ECIPHERTEXT_WRONG_VALUE: u64 = 10;


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

    // TODO: Describe in comment
    struct VeiledWithdrawalProof<phantom CoinType> has drop {
	x1: RistrettoPoint,
	x2: RistrettoPoint, 
	x3: RistrettoPoint,
	x4: RistrettoPoint,
	x5: RistrettoPoint,
	alpha1: Scalar,
	alpha2: Scalar,
	alpha3: Scalar,
	alpha4: Scalar,
    }

    /// Deserializes and returns a VeiledWithdrawalProof given its byte representation
    fun deserialize_withdrawal_proof<CoinType>(proof_bytes: vector<u8>): Option<VeiledWithdrawalProof<CoinType>> {
	assert!(vector::length<u8>(&proof_bytes) == 288, EBYTES_WRONG_LENGTH);
	let x1_bytes = vector::trim<u8>(&mut proof_bytes, 32);
	let x1 = ristretto255::new_point_from_bytes(x1_bytes);
	if (!std::option::is_some<RistrettoPoint>(&x1)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let x1 = std::option::extract<RistrettoPoint>(&mut x1);

	let x2_bytes = vector::trim(&mut proof_bytes, 32);	
	let x2 = ristretto255::new_point_from_bytes(x2_bytes);
	if (!std::option::is_some<RistrettoPoint>(&x2)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let x2 = std::option::extract<RistrettoPoint>(&mut x2);	

	let x3_bytes = vector::trim(&mut proof_bytes, 32);	
	let x3 = ristretto255::new_point_from_bytes(x3_bytes);
	if (!std::option::is_some<RistrettoPoint>(&x3)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let x3 = std::option::extract<RistrettoPoint>(&mut x3);

	let x4_bytes = vector::trim(&mut proof_bytes, 32);	
	let x4 = ristretto255::new_point_from_bytes(x4_bytes);
	if (!std::option::is_some<RistrettoPoint>(&x4)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let x4 = std::option::extract<RistrettoPoint>(&mut x4);

	let x5_bytes = vector::trim(&mut proof_bytes, 32);	
	let x5 = ristretto255::new_point_from_bytes(x5_bytes);
	if (!std::option::is_some<RistrettoPoint>(&x5)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let x5 = std::option::extract<RistrettoPoint>(&mut x5);

	let alpha1_bytes = vector::trim(&mut proof_bytes, 32);
	let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
	if (!std::option::is_some(&alpha1)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let alpha1 = std::option::extract(&mut alpha1);

	let alpha2_bytes = vector::trim(&mut proof_bytes, 32);
	let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
	if (!std::option::is_some(&alpha2)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let alpha2 = std::option::extract(&mut alpha2);

	let alpha3_bytes = vector::trim(&mut proof_bytes, 32);
	let alpha3 = ristretto255::new_scalar_from_bytes(alpha3_bytes);
	if (!std::option::is_some(&alpha3)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let alpha3 = std::option::extract(&mut alpha3);

	let alpha4_bytes = vector::trim(&mut proof_bytes, 32);
	let alpha4 = ristretto255::new_scalar_from_bytes(alpha4_bytes);
	if (!std::option::is_some(&alpha4)) {
	    return std::option::none<VeiledWithdrawalProof<CoinType>>()
	};
	let alpha4 = std::option::extract(&mut alpha4);

	std::option::some(VeiledWithdrawalProof {
	    x1, x2, x3, x4, x5, alpha1, alpha2, alpha3, alpha4
	})
    }

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// ElGamal ciphertext of a number of coins v \in [0, 2^{32}), an invariant
        /// that is enforced throughout the code
        private_value: Ciphertext,
    }

    /// A holder of a specific coin type and its associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A ElGamal ciphertext of a value v \in [0, 2^{32}), an invariant that is enforced throughout the code.
        private_balance: CompressedCiphertext,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
	pubkey: elgamal::Pubkey,
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

    /// Initializes a veiled coin store for the specified account. Requires an ElGamal public
    /// encryption key to be provided. The user must retain their corresponding secret key. 
    public entry fun register<CoinType>(sender: &signer, pubkey: vector<u8>) {
	let pubkey = elgamal::new_pubkey_from_bytes(pubkey);	
	register_internal<CoinType>(sender, std::option::extract(&mut pubkey));
    }

    /// Takes `amount` of `coin::Coin<CoinType>` coins from `sender`, wraps them inside a `VeiledCoin<CoinType>` and
    /// sends them back to `sender`.
    public entry fun wrap<CoinType>(sender: &signer, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        wrap_to<CoinType>(sender, signer::address_of(sender), amount)
    }

    /// Takes `amount` of `coin::Coin<CoinType>` coins from `sender`, wraps them inside a `VeiledCoin<CoinType>` and
    /// sends to `recipient`. Note that the returned veiled coin will not actually be private yet, since the amount is public here.
    public entry fun wrap_to<CoinType>(sender: &signer, recipient: address, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        let c = coin::withdraw<CoinType>(sender, amount);

        let vc = wrap_from_coin(c);

        deposit<CoinType>(recipient, vc)
    }

    /// Takes `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a coin::Coin<CoinType>,
    /// and sends them back to `sender`. Note that this function inherently leaks `amount`. Privacy of the remaining balance is maintained. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount.  
    // TODO: Remove range proof on transferred amount
    public entry fun unwrap<CoinType>(
	sender: &signer, 
	amount: u64, 
	range_proof_updated_balance: vector<u8>, 
	range_proof_transferred_amount: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
	unwrap_to<CoinType>(sender, signer::address_of(sender), amount, range_proof_updated_balance, range_proof_transferred_amount)
    }

    /// Takes `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a coin::Coin<CoinType>,
    /// and sends them to `recipient`. It is not possible for this function to be made private. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount.  
    public entry fun unwrap_to<CoinType>(
	sender: &signer, 
	recipient: address, 
	amount: u64, 
	range_proof_updated_balance_bytes: vector<u8>, 
	range_proof_transferred_amount_bytes: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
	let range_proof_updated_balance = bulletproofs::range_proof_from_bytes(range_proof_updated_balance_bytes);
	let range_proof_transferred_amount = bulletproofs::range_proof_from_bytes(range_proof_transferred_amount_bytes);
	
	let c = unwrap_to_coin<CoinType>(sender, amount, &range_proof_updated_balance, &range_proof_transferred_amount);
	coin::deposit<CoinType>(recipient, c);
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
	range_proof_transferred_amount: vector<u8>,
	sigma_proof: vector<u8>) acquires VeiledCoinStore {

	let private_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
	assert!(std::option::is_some(&private_withdraw_amount), EDESERIALIZATION_FAILED);
	let private_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
	assert!(std::option::is_some(&private_deposit_amount), EDESERIALIZATION_FAILED);
	let sigma_proof = deserialize_withdrawal_proof<CoinType>(sigma_proof);
	assert!(std::option::is_some(&sigma_proof), EDESERIALIZATION_FAILED);

	transfer_privately_to<CoinType>(
		sender,
		recipient,
		std::option::extract(&mut private_withdraw_amount),
		std::option::extract(&mut private_deposit_amount),
		&bulletproofs::range_proof_from_bytes(range_proof_updated_balance),
		&bulletproofs::range_proof_from_bytes(range_proof_transferred_amount),
		&std::option::extract(&mut sigma_proof),
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

    /// Given an address, returns the public key in the VeiledCoinStore associated with that address
    fun get_pubkey_from_addr<CoinType>(addr: address): Pubkey acquires VeiledCoinStore {
	assert!(has_veiled_coin_store<CoinType>(addr), EVEILED_COIN_STORE_NOT_PUBLISHED);
	let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(addr);
	coin_store.pubkey
    }

    /// Verifies a sigma proof needed to perform a private withdrawal. The relation is descirbed on
    /// page 14 of the Zether paper: https://crypto.stanford.edu/~buenz/papers/zether.pdf
    /// TODO: Finish description
    fun verify_withdrawal_sigma_protocol<CoinType>(sender_pubkey: &elgamal::Pubkey, recipient_pubkey: &Pubkey, balance: &Ciphertext, withdrawal_ct: &Ciphertext, deposit_ct: &Ciphertext, proof: &VeiledWithdrawalProof<CoinType>) {
	let sender_pubkey_point = elgamal::get_point_from_pubkey(sender_pubkey);
	let recipient_pubkey_point = elgamal::get_point_from_pubkey(recipient_pubkey);
	let (big_c, d) = elgamal::ciphertext_as_points(withdrawal_ct);
	let (bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
	let (c_L, c_R) = elgamal::ciphertext_as_points(balance);

	// c <- H(g,y,\bar{y},C_L,C_R,C,D,\bar{C},X_1,X_2,X_3,X_4,X_5)
	let hash_input = vector::empty<u8>();

	let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
	vector::append<u8>(&mut hash_input, basepoint_bytes);

	let y = elgamal::get_compressed_point_from_pubkey(sender_pubkey);
	let y_bytes = ristretto255::point_to_bytes(&y);
	vector::append<u8>(&mut hash_input, y_bytes);

	let y_bar = elgamal::get_compressed_point_from_pubkey(recipient_pubkey);
	let y_bar_bytes = ristretto255::point_to_bytes(&y_bar);
	vector::append<u8>(&mut hash_input, y_bar_bytes);

	let c_L_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c_L));
	vector::append<u8>(&mut hash_input, c_L_bytes);

	let c_R_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(c_R));
	vector::append<u8>(&mut hash_input, c_R_bytes);

	let big_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(big_c));
	vector::append<u8>(&mut hash_input, big_c_bytes);

	let d_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(d));
	vector::append<u8>(&mut hash_input, d_bytes);

	let bar_c_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(bar_c));
	vector::append<u8>(&mut hash_input, bar_c_bytes);

	let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
	vector::append<u8>(&mut hash_input, x_1_bytes);

	let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
	vector::append<u8>(&mut hash_input, x_2_bytes);

	let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
	vector::append<u8>(&mut hash_input, x_3_bytes);

	let x_4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x4));
	vector::append<u8>(&mut hash_input, x_4_bytes);

	let x_5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x5));
	vector::append<u8>(&mut hash_input, x_5_bytes);

	let c = ristretto255::new_scalar_from_sha2_512(hash_input);

	// c * D + X1 =? \alpha_1 * g
	let d_acc = ristretto255::point_mul(d, &c);	
	ristretto255::point_add_assign(&mut d_acc, &proof.x1);
	let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
	assert!(ristretto255::point_equals(&d_acc, &g_alpha1), ESIGMA_PROTOCOL_VERIFY_FAILED);

	// c * y + X2 =? \alpha_2 * g
	let y_times_c = ristretto255::point_mul(&sender_pubkey_point, &c);
	ristretto255::point_add_assign(&mut y_times_c, &proof.x2);
	let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
	assert!(ristretto255::point_equals(&y_times_c, &g_alpha2), ESIGMA_PROTOCOL_VERIFY_FAILED);

	let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3); 
	// c * C + X3 =? \alpha_3 * g + \alpha_1 * y
	let big_c = ristretto255::point_mul(big_c, &c);
	ristretto255::point_add_assign(&mut big_c, &proof.x3);
	let y_alpha1 = ristretto255::point_mul(&sender_pubkey_point, &proof.alpha1);
	ristretto255::point_add_assign(&mut y_alpha1, &g_alpha3);
	assert!(ristretto255::point_equals(&big_c, &y_alpha1), ESIGMA_PROTOCOL_VERIFY_FAILED);

	// c * \bar{C} + X4 =? \alpha_3 * g + \alpha_1 * \bar{y}
	let bar_c = ristretto255::point_mul(bar_c, &c);
	ristretto255::point_add_assign(&mut bar_c, &proof.x4);
	let bar_y_alpha1 = ristretto255::point_mul(&recipient_pubkey_point, &proof.alpha1);
	ristretto255::point_add_assign(&mut bar_y_alpha1, &g_alpha3);
	assert!(ristretto255::point_equals(&bar_c, &bar_y_alpha1), ESIGMA_PROTOCOL_VERIFY_FAILED);

	// c * (C_L + -C) + X5 =? \alpha_4 * g + \alpha_2 * (C_R + -D)
	let neg_C = ristretto255::point_neg(&big_c);
	ristretto255::point_add_assign(&mut neg_C, c_L);
	ristretto255::point_mul_assign(&mut neg_C, &c);
	ristretto255::point_add_assign(&mut neg_C, &proof.x5);
	let neg_D = ristretto255::point_neg(d);
	ristretto255::point_add_assign(&mut neg_D, c_R);
	ristretto255::point_mul_assign(&mut neg_D, &proof.alpha2);
	let g_alpha4 = ristretto255::basepoint_mul(&proof.alpha4);
	ristretto255::point_add_assign(&mut neg_D, &g_alpha4);
	assert!(ristretto255::point_equals(&neg_C, &neg_D), ESIGMA_PROTOCOL_VERIFY_FAILED);
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

    /// Initializes a veiled coin store for the specified account. Requires an ElGamal public
    /// encryption key to be provided. The user must retain their corresponding secret key. 
    public fun register_internal<CoinType>(account: &signer, pubkey: elgamal::Pubkey) {
        let account_addr = signer::address_of(account);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED),
        );

        let coin_store = VeiledCoinStore<CoinType> {
            private_balance: elgamal::new_ciphertext_from_compressed(ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed()),
            deposit_events: account::new_event_handle<DepositEvent>(account),
            withdraw_events: account::new_event_handle<WithdrawEvent>(account),
	    pubkey: pubkey,
        };
        move_to(account, coin_store);
    }

    /// Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.
    ///
    /// WARNING: Fundamentally, there is no way to hide the value of the coin being minted here.
    public fun wrap_from_coin<CoinType>(c: Coin<CoinType>): VeiledCoin<CoinType> acquires VeiledCoinMinter {
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

    /// Removes `amount` coins from the VeiledCoinStore balance of `sender`, by subtracting `amount_ct` from
    /// their VeiledCoinStore balance. It is not possible for this function to be made private. Requires a range
    /// proof on the new balance of the sender, to ensure the sender has enough money to send, in addition to a 
    /// range proof on the transferred amount.
    public fun unwrap_to_coin<CoinType>(
	sender: &signer, 
	amount: u64,
	range_proof_updated_balance: &RangeProof,
	range_proof_transferred_amount: &RangeProof): Coin<CoinType> acquires VeiledCoinStore, VeiledCoinMinter {
	// resource account signer should exist as wrap_to_coin should already have been called
	let rsrc_acc_signer = get_resource_account_signer();
	let scalar_amount = new_scalar_from_u64(amount);
	let computed_ct = elgamal::new_ciphertext_no_randomness(&scalar_amount);

	let sender_addr = signer::address_of(sender);
	let sender_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);

	withdraw(sender, computed_ct, sender_coin_store, range_proof_updated_balance, range_proof_transferred_amount);
	coin::withdraw(&rsrc_acc_signer, amount)
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
        recipient_addr: address,
        private_withdraw_amount: Ciphertext,
	private_deposit_amount: Ciphertext,
        range_proof_updated_balance: &RangeProof,
	range_proof_transferred_amount: &RangeProof,
	proof: &VeiledWithdrawalProof<CoinType>)
    acquires VeiledCoinStore {
	let sender_addr = signer::address_of(sender);

	// get_pubkey_from_addr checks if each account has a VeiledCoinStore published
	let sender_pubkey = get_pubkey_from_addr<CoinType>(sender_addr);
	let recipient_pubkey = get_pubkey_from_addr<CoinType>(recipient_addr);
	let sender_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);

	verify_withdrawal_sigma_protocol(&sender_pubkey, &recipient_pubkey, &elgamal::decompress_ciphertext(&sender_coin_store.private_balance), &private_withdraw_amount, &private_deposit_amount, proof);

        withdraw<CoinType>(sender, private_withdraw_amount, sender_coin_store, range_proof_updated_balance, range_proof_transferred_amount);
	let vc = VeiledCoin<CoinType> { private_value: private_deposit_amount };

        deposit(recipient_addr, vc);
    }

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
	coin_store: &mut VeiledCoinStore<CoinType>,
        range_proof_updated_balance: &RangeProof,
	range_proof_transferred_amount: &RangeProof,
    ) {
        let account_addr = signer::address_of(account);
        assert!(
            has_veiled_coin_store<CoinType>(account_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

	let pubkey = &coin_store.pubkey;

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
        assert!(bulletproofs::verify_range_proof_elgamal(&private_balance, range_proof_updated_balance, pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);
	assert!(bulletproofs::verify_range_proof_elgamal(&withdraw_amount, range_proof_transferred_amount, pubkey, MAX_BITS_IN_VALUE, VEILED_COIN_DST), ERANGE_PROOF_VERIFICATION_FAILED);

        coin_store.private_balance = elgamal::compress_ciphertext(&private_balance);
    }

    //
    // Tests
    //

    // TODO: Update tests

    /*const SOME_RANDOMNESS_1: vector<u8> = x"e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c";

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
    }*/
}
