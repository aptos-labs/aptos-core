/// This module provides a veiled coin type, denoted `VeiledCoin<T>` that hides the value/denomination of a coin.
/// Importantly, although veiled transactions hide the amount of coins sent they still leak the sender and recipient.
///
/// ## Limitations
///
/// **WARNING:** This module is **experimental**! It is *NOT* production-ready. Specifically:
///
///  1. deploying this module will likely lead to lost funds
///  2. this module has not been cryptographically-audited
///  3. the current implementation is vulnerable to _front-running attacks_ as described in the Zether paper [BAZB20].
///  4. there is no integration with wallet software which, for veiled accounts, must maintain an additional ElGamal
///    encryption keypair
///  5. there is no support for rotating the ElGamal encryption public key of a veiled account
///
/// Another limitation is veiled coin amounts must be speicified as `u32`'s rather than `u64`'s as would be typical for
/// normal coins in the Aptos framework.
///
/// TODO: Describe how the `u32` works.
///
/// ## Terminology
///
/// 1. *Veiled coin*: a coin whose value is secret; i.e., it is encrypted under the owner's public key
///
/// 2. *Veiled amount*: any amount that is secret; i.e., encrypted under some public key
///
/// 3. *Veiled transaction*: a transaction that hides its amount transferred; i.e., a transaction whose amount is veiled
///
/// 4. *Veiled balance*: unlike a normal balance, a veiled balance is secret; i.e., it is encrypted under the account's
///    public key
///
/// ## Implementation details
///
/// This module leverages a so-called "resource account," which helps us mint a `VeiledCoin<T>` from a
/// normal `coin::Coin<T>` by transferring this latter coin into a `coin::CoinStore<T>` stored in the
/// resource account.
///
/// Later on, when someone wants to convert their `VeiledCoin<T>` into a normal `coin::Coin<T>`,
/// the resource account can be used to transfer out the normal from its coin store. Transfering out a coin like this
/// requires a `signer` for the resource account, which the `veiled_coin` module can obtain via a `SignerCapability`.
///
/// ## References
///
/// [BAZB20] Zether: Towards Privacy in a Smart Contract World; by Bunz, Benedikt and Agrawal, Shashank and Zamani,
/// Mahdi and Boneh, Dan; in Financial Cryptography and Data Security; 2020
module veiled_coin::veiled_coin {
    use std::error;
    use std::signer;
    use std::vector;
    use std::option::Option;

    use aptos_std::elgamal::Self;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar};

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::bulletproofs::RangeProof;
    use aptos_std::bulletproofs;

    #[test_only]
    use std::features;

    //
    // Errors
    //

    /// The range proof system does not support proofs for any number \in [0, 2^{32})
    const ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE : u64 = 1;

    /// A range proof failed to verify.
    const ERANGE_PROOF_VERIFICATION_FAILED : u64 = 2;

    /// Account already has `VeiledCoinStore<CoinType>` registered.
    const EVEILED_COIN_STORE_ALREADY_PUBLISHED: u64 = 3;

    /// Account hasn't registered `VeiledCoinStore<CoinType>`.
    const EVEILED_COIN_STORE_NOT_PUBLISHED: u64 = 4;

    /// Not enough coins to complete transaction.
    const EINSUFFICIENT_BALANCE: u64 = 5;

    /// Failed deserializing bytes into either ElGamal ciphertext or Sigma protocol proof.
    const EDESERIALIZATION_FAILED: u64 = 6;

    /// Byte vector given for deserialization was the wrong length.
    const EBYTES_WRONG_LENGTH: u64 = 7;

    /// Sigma protocol proof for withdrawals did not verify.
    const ESIGMA_PROTOCOL_VERIFY_FAILED: u64 = 8;

    /// Tried cutting out more elements than are in the vector via `cut_vector`.
    const EVECTOR_CUT_TOO_LARGE: u64 = 9;

    //
    // Constants
    //

    /// The maximum number of bits used to represent a coin's value.
    const MAX_BITS_IN_VALUE : u64 = 32;

    /// The domain separation tag (DST) used for the Bulletproofs prover.
    const VEILED_COIN_DST : vector<u8> = b"AptosVeiledCoin/BulletproofRangeProof";

    /// The domain separation tag (DST) used in the Fiat-Shamir transform of our Sigma protocol.
    const FIAT_SHAMIR_SIGMA_DST : vector<u8> = b"AptosVeiledCoin/WithdrawalProofFiatShamir";

    //
    // Structs
    //

    /// Main structure representing a coin in an account's custody.
    struct VeiledCoin<phantom CoinType> {
        /// ElGamal ciphertext which encrypts the number of coins $v \in [0, 2^{32})$. This $[0, 2^{32})$ range invariant
        /// is enforced throughout the code via Bulletproof-based ZK range proofs.
        veiled_amount: elgamal::Ciphertext,
    }

    /// A holder of a specific coin type and its associated event handles.
    /// These are kept in a single resource to ensure locality of data.
    struct VeiledCoinStore<phantom CoinType> has key {
        /// A ElGamal ciphertext of a value $v \in [0, 2^{32})$, an invariant that is enforced throughout the code.
        veiled_balance: elgamal::CompressedCiphertext,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
        pk: elgamal::CompressedPubkey,
    }

    /// Holds an `account::SignerCapability` for the resource account created when initializing this module. This
    /// resource account houses a `coin::CoinStore<T>` for every type of coin `T` that is veiled.
    struct VeiledCoinMinter has store, key {
        signer_cap: account::SignerCapability,
    }

    /// A cryptographic proof that ensures correctness of a veiled-to-veiled coin transfer.
    struct VeiledTransferProof<phantom CoinType> has drop {
        new_balance_proof: RangeProof,
        veiled_amount_proof: RangeProof,
        sigma_proof: SigmaProof<CoinType>,
    }

    /// A Sigma protocol proof used as part of a `VeiledTransferProof`.
    /// (A more detailed description can be found in `verify_withdrawal_sigma_protocol`.)
    struct SigmaProof<phantom CoinType> has drop {
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

    /// Event emitted when some amount of veiled coins were deposited into an account.
    struct DepositEvent has drop, store {
        // We cannot leak any information about how much has been deposited.
    }

    /// Event emitted when some amount of veiled coins were withdrawn from an account.
    struct WithdrawEvent has drop, store {
        // We cannot leak any information about how much has been withdrawn.
    }

    //
    // Module initialization, done only once when this module is first published on the blockchain
    //

    /// Initializes a so-called "resource" account which will maintain a `coin::CoinStore<T>` resource for all `Coin<T>`'s
    /// that have been converted into a `VeiledCoin<T>`.
    fun init_module(deployer: &signer) {
        assert!(
            bulletproofs::get_max_range_bits() >= MAX_BITS_IN_VALUE,
            error::internal(ERANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE)
        );

        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // transfer `Coin<T>`'s into its `CoinStore<T>` before minting a `VeiledCoin<T>`.
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

    /// Initializes a veiled coin store for the specified `user` account with that user's ElGamal encryption public key.
    /// Importantly, the user's wallet must retain their corresponding secret key.
    public entry fun register<CoinType>(user: &signer, pk: vector<u8>) {
        let pk = elgamal::new_pubkey_from_bytes(pk);
        register_internal<CoinType>(user, std::option::extract(&mut pk));
    }

    /// Sends a *public* `amount` of normal coins from `sender` to the `recipient`'s veiled balance.
    ///
    /// **WARNING:** This function *leaks* the transferred `amount`, since it is given as a public input.
    public entry fun veil_to<CoinType>(
        sender: &signer, recipient: address, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore
    {
        let c = coin::withdraw<CoinType>(sender, amount);

        let vc = unveiled_to_veiled_coin(c);

        deposit<CoinType>(recipient, vc)
    }

    /// Like `veil_to` except the `sender` is also the recipient.
    ///
    /// This function can be used by the `sender` to initialize his veiled balance to a *public* value.
    ///
    /// **WARNING:** The initialized balance is *leaked*, since its initialized `amount` is public here.
    public entry fun veil<CoinType>(sender: &signer, amount: u64) acquires VeiledCoinMinter, VeiledCoinStore {
        veil_to<CoinType>(sender, signer::address_of(sender), amount)
    }

    /// Takes a *public* `amount` of `VeiledCoin<CoinType>` coins from `sender`, unwraps them to a `coin::Coin<CoinType>`,
    /// and sends them to `recipient`. Maintains secrecy of `sender`'s new balance.
    ///
    /// Requires a range proof on the new balance of the sender, to ensure the sender has enough money to send.
    /// No range proof is necessary for the `amount`, which is given as a public `u32` value.
    ///
    /// **WARNING:** This *leaks* the transferred `amount`, since it is a public `u32` argument.
    public entry fun unveil_to<CoinType>(
        sender: &signer,
        recipient: address,
        amount: u32,
        range_proof_new_balance: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
        let range_proof_new_balance = bulletproofs::range_proof_from_bytes(range_proof_new_balance);

        let c = veiled_to_unveiled_coin<CoinType>(sender, amount, &range_proof_new_balance);
        coin::deposit<CoinType>(recipient, c);
    }

    /// Like `unveil_to`, except the `sender` is also the recipient.
    public entry fun unveil<CoinType>(
        sender: &signer,
        amount: u32,
        range_proof_new_balance: vector<u8>) acquires VeiledCoinStore, VeiledCoinMinter
    {
        unveil_to<CoinType>(sender, signer::address_of(sender), amount, range_proof_new_balance)
    }

    /// Sends a *veiled* `amount` from `sender` to `recipient`. After this call, the balance of the `sender`
    /// and `recipient` remains (or becomes) secret.
    ///
    /// The sent amount remains secret; It is encrypted both under the sender's PK (in `withdraw_ct`) and under the
    /// recipient's PK (in `deposit_ct`) using the *same* ElGamal randomness.
    ///
    /// Requires a `VeiledTransferProof`; i.e.:
    /// 1. A range proof on the new balance of the sender, to ensure the sender has enough money to send (in
    ///    `range_proof_new_balance`),
    /// 2. A range proof on the transferred amount in `withdraw_ct`, to ensure the sender won't create coins out of thin
    ///    air (in `range_proof_veiled_amount`),
    /// 3. A Sigma protocol to prove that 'veiled_withdraw_amount' encrypts the same veiled amount as
    ///    'veiled_deposit_amount' with the same randomness (in `sigma_proof_bytes`).
    public entry fun fully_veiled_transfer<CoinType>(
        sender: &signer,
        recipient: address,
        withdraw_ct: vector<u8>,
        deposit_ct: vector<u8>,
        range_proof_new_balance: vector<u8>,
        range_proof_veiled_amount: vector<u8>,
        sigma_proof_bytes: vector<u8>) acquires VeiledCoinStore
    {
        let veiled_withdraw_amount = elgamal::new_ciphertext_from_bytes(withdraw_ct);
        assert!(std::option::is_some(&veiled_withdraw_amount), error::invalid_argument(EDESERIALIZATION_FAILED));

        let veiled_deposit_amount = elgamal::new_ciphertext_from_bytes(deposit_ct);
        assert!(std::option::is_some(&veiled_deposit_amount), error::invalid_argument(EDESERIALIZATION_FAILED));

        // This Sigma protocol proofs proves that `veiled_withdraw_amount` encrypts the same value using the same
        // randomness as `veiled_deposit_amount` (i.e., the amount being transferred). These two ciphertexts are
        // required as we need to update both the sender's and the recipient's balances, which use different public keys
        // and so must be updated with ciphertexts encrypted under their respective public keys.
        let sigma_proof = deserialize_sigma_proof<CoinType>(sigma_proof_bytes);
        assert!(std::option::is_some(&sigma_proof), error::invalid_argument(EDESERIALIZATION_FAILED));

        // Requires a range proof on the new balance of the sender, to ensure the sender has enough money to send, in
        // addition to a  range proof on the transferred amount.
        let new_balance_proof = bulletproofs::range_proof_from_bytes(range_proof_new_balance);
        let veiled_amount_proof = bulletproofs::range_proof_from_bytes(range_proof_veiled_amount);

        let transfer_proof = VeiledTransferProof {
            new_balance_proof,
            veiled_amount_proof,
            sigma_proof: std::option::extract(&mut sigma_proof)
        };

        fully_veiled_transfer_internal<CoinType>(
            sender,
            recipient,
            std::option::extract(&mut veiled_withdraw_amount),
            std::option::extract(&mut veiled_deposit_amount),
            &transfer_proof,
        )
    }

    //
    // Public functions.
    //

    /// Returns `true` if `addr` is registered to receive veiled coins of `CoinType`.
    public fun has_veiled_coin_store<CoinType>(addr: address): bool {
        exists<VeiledCoinStore<CoinType>>(addr)
    }

    /// Returns the ElGamal encryption of the value of `coin`.
    public fun veiled_amount<CoinType>(coin: &VeiledCoin<CoinType>): &elgamal::Ciphertext {
        &coin.veiled_amount
    }

    /// Returns the ElGamal encryption of the veiled balance of `owner` for the provided `CoinType`.
    public fun veiled_balance<CoinType>(owner: address): elgamal::CompressedCiphertext acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(owner),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        borrow_global<VeiledCoinStore<CoinType>>(owner).veiled_balance
    }

    /// Given an address `addr`, returns the ElGamal encryption public key associated with that address
    public fun encryption_public_key<CoinType>(addr: address): elgamal::CompressedPubkey acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED)
        );

        borrow_global_mut<VeiledCoinStore<CoinType>>(addr).pk
    }

    /// Like `register`, but the public key is parsed in an `elgamal::CompressedPubkey` struct.
    public fun register_internal<CoinType>(user: &signer, pk: elgamal::CompressedPubkey) {
        let account_addr = signer::address_of(user);
        assert!(
            !has_veiled_coin_store<CoinType>(account_addr),
            error::already_exists(EVEILED_COIN_STORE_ALREADY_PUBLISHED),
        );

        // Note: There is no way to find an ElGamal SK such that the `(0_G, 0_G)` ciphertext below decrypts to a non-zero
        // value. We'd need to have `(r * G, v * G + r * pk) = (0_G, 0_G)`, which implies `r = 0` for any choice of PK/SK.
        // Thus, we must have `v * G = 0_G`, which implies `v = 0`.

        let coin_store = VeiledCoinStore<CoinType> {
            veiled_balance: elgamal::ciphertext_from_compressed_points(
                ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed()),
            deposit_events: account::new_event_handle<DepositEvent>(user),
            withdraw_events: account::new_event_handle<WithdrawEvent>(user),
            pk,
        };
        move_to(user, coin_store);
    }

    /// Mints a veiled coin from a normal coin, shelving the normal coin into the resource account's coin store.
    ///
    /// **WARNING:** Fundamentally, there is no way to hide the value of the coin being minted here.
    public fun unveiled_to_veiled_coin<CoinType>(c: Coin<CoinType>): VeiledCoin<CoinType> acquires VeiledCoinMinter {
        // If there is no CoinStore<CoinType> in the resource account, create one.
        let rsrc_acc_signer = get_resource_account_signer();
        let rsrc_acc_addr = signer::address_of(&rsrc_acc_signer);
        if (!coin::is_account_registered<CoinType>(rsrc_acc_addr)) {
            coin::register<CoinType>(&rsrc_acc_signer);
        };

        // Move the normal coin into the coin store, so we can mint a veiled coin.
        // (There is no other way to drop a normal coin, for safety reasons, so moving it into a coin store is
        //  the only option.)
        let value = ristretto255::new_scalar_from_u64(coin::value(&c));
        coin::deposit(rsrc_acc_addr, c);

        VeiledCoin<CoinType> {
            veiled_amount: elgamal::new_ciphertext_no_randomness(&value)
        }
    }

    /// Removes a *public* `amount` of veiled coins from `sender` and returns them as a normal `coin::Coin`.
    ///
    /// Requires a ZK range proof on the new balance of the `sender`, to ensure the `sender` has enough money to send.
    /// Since the `amount` is public, no ZK range proof on it is required.
    ///
    /// **WARNING:** This function *leaks* the public `amount`.
    public fun veiled_to_unveiled_coin<CoinType>(
        sender: &signer,
        amount: u32,
        new_balance_proof: &RangeProof): Coin<CoinType> acquires VeiledCoinStore, VeiledCoinMinter
    {
        // TODO: Is this casting enough? I thought we will shift the bits
        let scalar_amount = ristretto255::new_scalar_from_u64((amount as u64));
        let veiled_amount = elgamal::new_ciphertext_no_randomness(&scalar_amount);

        let addr = signer::address_of(sender);

        assert!(has_veiled_coin_store<CoinType>(addr), error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED));
        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(addr);

        // Since `veiled_amount` was created from a `u32` public `amount`, no ZK range proof is needed for it.
        withdraw(veiled_amount, coin_store, new_balance_proof, &std::option::none());

        // Note: If the above `withdraw` aborts, the whole TXN aborts, so there are no atomicity issues.
        coin::withdraw(&get_resource_account_signer(), (amount as u64))
    }


    /// Like `fully_veiled_transfer`, except the ciphertext and proofs have been deserialized into their respective structs.
    public fun fully_veiled_transfer_internal<CoinType>(
        sender: &signer,
        recipient_addr: address,
        veiled_withdraw_amount: elgamal::Ciphertext,
        veiled_deposit_amount: elgamal::Ciphertext,
        transfer_proof: &VeiledTransferProof<CoinType>) acquires VeiledCoinStore
    {
        let sender_addr = signer::address_of(sender);

        let sender_pk = encryption_public_key<CoinType>(sender_addr);
        let recipient_pk = encryption_public_key<CoinType>(recipient_addr);

        // Note: The `get_pk_from_addr` call from above already asserts that `sender_addr` has a coin store.
        let sender_coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(sender_addr);

        // Checks that `veiled_withdraw_amount` and `veiled_deposit_amount` encrypt the same amount of coins, under the
        // sender and recipient's PKs, respectively, by verifying the sigma protocol proof in `transfer_proof`.
        sigma_protocol_verify(
            &sender_pk,
            &recipient_pk,
            &elgamal::decompress_ciphertext(&sender_coin_store.veiled_balance),
            &veiled_withdraw_amount,
            &veiled_deposit_amount,
            &transfer_proof.sigma_proof);

        // Verifies the range proofs in `transfer_proof` and withdraws `veiled_withdraw_amount` from the `sender`'s account.
        withdraw<CoinType>(
            veiled_withdraw_amount,
            sender_coin_store,
            &transfer_proof.new_balance_proof,
            &std::option::some(transfer_proof.veiled_amount_proof));

        // Creates a new veiled coin for the recipient.
        let vc = VeiledCoin<CoinType> { veiled_amount: veiled_deposit_amount };

        // Deposits `veiled_deposit_amount` into the recipient's account
        // (Note, if this aborts, the whole transaction aborts, so we do not need to worry about atomicity.)
        deposit(recipient_addr, vc);
    }

    /// Deposits a veiled coin at address `to_addr`.
    public fun deposit<CoinType>(to_addr: address, coin: VeiledCoin<CoinType>) acquires VeiledCoinStore {
        assert!(
            has_veiled_coin_store<CoinType>(to_addr),
            error::not_found(EVEILED_COIN_STORE_NOT_PUBLISHED),
        );

        let coin_store = borrow_global_mut<VeiledCoinStore<CoinType>>(to_addr);

        // Fetch the veiled balance
        let veiled_balance = elgamal::decompress_ciphertext(&coin_store.veiled_balance);

        // Subtract the veiled amount from it, homomorphically
        elgamal::ciphertext_add_assign(&mut veiled_balance, &coin.veiled_amount);

        // Update the veiled balance
        coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

        // Make sure the veiled coin is dropped so it cannot be double spent
        drop_veiled_coin(coin);

        // Once successful, emit an event that a veiled deposit occurred.
        event::emit_event<DepositEvent>(
            &mut coin_store.deposit_events,
            DepositEvent {},
        );
    }

    /// Withdraws a `veiled_amount` of coins from the specified coin store. Let `balance` denote its current
    /// *veiled* balance.
    ///
    /// **WARNING:** This function assumes that `veiled_amount` is correctly encrypted under the sender's PK. This
    /// is the case when either (1) the amount was veiled correctly from a public value or (2) a Sigma protocol proof
    /// over `veiled_amount` verified successfully.
    ///
    /// Always requires a ZK range proof `new_balance_proof` on `balance - amount`. When the veiled amount was NOT
    /// created from a public value, additionally requires a ZK range proof `veiled_amount_proof` on `amount`.
    public fun withdraw<CoinType>(
        veiled_amount: elgamal::Ciphertext,
        coin_store: &mut VeiledCoinStore<CoinType>,
        new_balance_proof: &RangeProof,
        veiled_amount_proof: &Option<RangeProof>)
    {
        // Fetch the ElGamal public key of the veiled account
        let pk = &coin_store.pk;

        // Fetch the veiled balance of the veiled account
        let veiled_balance = elgamal::decompress_ciphertext(&coin_store.veiled_balance);

        // Update the account's veiled balance by homomorphically subtracting the veiled amount from the veiled balance.
        elgamal::ciphertext_sub_assign(&mut veiled_balance, &veiled_amount);

        // This function checks if it is possible to withdraw a veiled `amount` from a veiled `bal`, obtaining a new
        // veiled balance `new_bal = bal - amount`. It maintains an invariant that `new_bal \in [0, 2^{32})` as follows.
        //
        //  1. We assume (by the invariant) that `bal \in [0, 2^{32})`.
        //
        //  2. We verify a ZK range proof that `amount \in [0, 2^{32})`. Otherwise, a sender could set `amount = p-1`
        //     where `p` is the order of the scalar field, which would give `new_bal = bal - (p-1) mod p = bal + 1`.
        //     Therefore, a malicious spender could create coins out of thin air for themselves.
        //
        //  3. We verify a ZK range proof that `new_bal \in [0, 2^{32})`. Otherwise, a sender could set `amount = bal + 1`,
        //     which would satisfy condition (2) from above but would give `new_bal = bal - (bal + 1) = -1`. Therefore,
        //     a malicious spender could spend more coins than they have.
        //
        // Altogether, these checks ensure that `bal - amount >= 0` (as integers) and therefore that `bal >= amount`
        // (again, as integers).
        //
        // When the caller of this function created the `veiled_amount` from a public `u32` value, the
        // `veiled_amount_proof` range proof is no longer necessary since the caller guarantees that condition (2) from
        // above holds.

        // Checks range condition (3)
        assert!(
            bulletproofs::verify_range_proof_elgamal(
                &veiled_balance,
                new_balance_proof,
                pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );

        // Checks range condition (2), if the veiled amount did not originate from a public amount
        if (std::option::is_some(veiled_amount_proof)) {
            assert!(
                bulletproofs::verify_range_proof_elgamal(
                    &veiled_amount,
                    std::option::borrow(veiled_amount_proof),
                    pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST
                ),
                error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
            );
        };

        // Update the veiled balance to reflect the veiled withdrawal
        coin_store.veiled_balance = elgamal::compress_ciphertext(&veiled_balance);

        // Once everything succeeds, emit an event to indicate a veiled withdrawal occurred
        event::emit_event<WithdrawEvent>(
            &mut coin_store.withdraw_events,
            WithdrawEvent { },
        );
    }

    //
    // Private functions.
    //

    /// Given a vector `vec`, removes the last `cut_len` elements of `vec` and returns them in order. (This function
    /// exists because we did not like the interface of `std::vector::trim`.)
    fun cut_vector<T>(vec: &mut vector<T>, cut_len: u64): vector<T> {
        let len = vector::length(vec);
        let res = vector::empty();
        assert!(len >= cut_len, error::out_of_range(EVECTOR_CUT_TOO_LARGE));
        while (cut_len > 0) {
            vector::push_back(&mut res, vector::pop_back(vec));
            cut_len = cut_len - 1;
        };
        vector::reverse(&mut res);
        res
    }

    /// Returns a signer for the resource account storing all the normal coins that have been veiled.
    fun get_resource_account_signer(): signer acquires VeiledCoinMinter {
        account::create_signer_with_capability(&borrow_global<VeiledCoinMinter>(@veiled_coin).signer_cap)
    }

    /// Used internally to drop veiled coins that were split or joined.
    fun drop_veiled_coin<CoinType>(c: VeiledCoin<CoinType>) {
        let VeiledCoin<CoinType> { veiled_amount: _ } = c;
    }

    /// Deserializes and returns a `SigmaProof` given its byte representation.
    /// TODO: reference the sigma protocol here
    /// Elements at the end of the `SigmaProof` struct are expected to be at the start
    /// of the byte vector, and serialized using the serialization formats in the
    /// `ristretto255` module.
    fun deserialize_sigma_proof<CoinType>(proof_bytes: vector<u8>): Option<SigmaProof<CoinType>> {
        if (vector::length<u8>(&proof_bytes) != 288) {
            return std::option::none<SigmaProof<CoinType>>()
        };

        let x1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x1 = ristretto255::new_point_from_bytes(x1_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x1)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x1 = std::option::extract<RistrettoPoint>(&mut x1);

        let x2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x2 = ristretto255::new_point_from_bytes(x2_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x2)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x2 = std::option::extract<RistrettoPoint>(&mut x2);

        let x3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x3 = ristretto255::new_point_from_bytes(x3_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x3)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x3 = std::option::extract<RistrettoPoint>(&mut x3);

        let x4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x4 = ristretto255::new_point_from_bytes(x4_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x4)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x4 = std::option::extract<RistrettoPoint>(&mut x4);

        let x5_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let x5 = ristretto255::new_point_from_bytes(x5_bytes);
        if (!std::option::is_some<RistrettoPoint>(&x5)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let x5 = std::option::extract<RistrettoPoint>(&mut x5);

        let alpha1_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha1 = ristretto255::new_scalar_from_bytes(alpha1_bytes);
        if (!std::option::is_some(&alpha1)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha1 = std::option::extract(&mut alpha1);

        let alpha2_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha2 = ristretto255::new_scalar_from_bytes(alpha2_bytes);
        if (!std::option::is_some(&alpha2)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha2 = std::option::extract(&mut alpha2);

        let alpha3_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha3 = ristretto255::new_scalar_from_bytes(alpha3_bytes);
        if (!std::option::is_some(&alpha3)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha3 = std::option::extract(&mut alpha3);

        let alpha4_bytes = cut_vector<u8>(&mut proof_bytes, 32);
        let alpha4 = ristretto255::new_scalar_from_bytes(alpha4_bytes);
        if (!std::option::is_some(&alpha4)) {
            return std::option::none<SigmaProof<CoinType>>()
        };
        let alpha4 = std::option::extract(&mut alpha4);

        std::option::some(SigmaProof {
            x1, x2, x3, x4, x5, alpha1, alpha2, alpha3, alpha4
        })
    }

    /// Verifies a Sigma protocol proof necessary to ensure correctness of a veiled transfer.
    ///
    /// The proof argues knowledge of a witness $w$ such that a specific relation $R(x; w)$ is satisfied, for a public
    /// statement $x$ known to the verifier (i.e., known to the validators). We describe this relation below.
    ///
    /// The secret witness $w$ in this relation, known only to the sender of the TXN, consists of:
    ///  - $v$, the amount being transferred
    ///  - $sk$, the sender's SK
    ///  - $b$, the sender's new balance (after withdrawing $v$)
    ///  - $r$, ElGamal encryption randomness
    ///
    /// (Note that the Sigma protocol's zero-knowledge property ensures the witness is not revealed.)
    ///
    /// The public statement $w$ in this relation consists of:
    ///  - $Y$, the sender's PK
    ///  - $Y'$, the recipient's PK
    ///  - $(B_L, B_R)$, the sender's encrypted balance (before withdrawing $v$)
    ///  - $(C, D)$, the ElGamal encryption of $v$ under the sender's PK
    ///  - $(C', D)$, the ElGamal encryption of $v$ under the recipient's PK
    ///
    ///
    /// The relation, at a high level, ensures that the sender withdrew $v$ from their encrypted balance $(B_L, B_R)$
    /// and created two ciphertexts $(C, D)$ and $(C', D)$ encrypting $v$ under the sender's PK and recipient's PK, respectively.:
    ///
    /// ```
    /// R(
    ///     x = [ Y, Y', (B_L, B_R), (C, C', D), G]
    ///     w = [ sk, v, b, r ]
    /// ) = {
    ///     C = v * G + r * Y
    ///     C' = v * G + r * Y'
    ///     D = r * G
    ///     B_L - C = b * G + sk * (B_R - D)
    ///     Y = sk * G
    /// }
    /// ```
    ///
    /// A relation similar to this is also described on page 14 of the Zether paper [BAZB20] (just replace $(B_L, B_R)$
    /// -> $(C_L, C_R)$, $G$ -> $g$, $C'$ -> $\bar{C}$, $Y$ -> $y$, $Y'$ -> $\bar{y}$, $b$ -> $b'$, replace $v$ -> $b^*$).
    ///
    /// Specifically, this protocol proves that `withdraw_ct` and `deposit_ct` encrypt the same
    /// amount $v$ using the same randomness $r$, with `sender_pk` and `recipient_pk` respectively.
    ///
    /// It additionally proves that `sender_pk` was generated with the sender's secret key $sk$, and that `balance`
    /// equals the correct updated value $b$ once `withdraw_ct` has been subtracted from it.
    fun sigma_protocol_verify<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        balance_ct: &elgamal::Ciphertext,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        proof: &SigmaProof<CoinType>)
    {
        let sender_pk_point = elgamal::pubkey_to_point(sender_pk);
        let recipient_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let (big_c, d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c_L, c_R) = elgamal::ciphertext_as_points(balance_ct);

        // TODO: Can be optimized so we don't re-serialize the proof for Fiat-Shamir
        let c = sigma_protocol_fiat_shamir<CoinType>(
            sender_pk, recipient_pk,
            withdraw_ct, deposit_ct, balance_ct,
            &proof.x1, &proof.x2, &proof.x3, &proof.x4, &proof.x5);

        // c * D + X1 =? \alpha_1 * g
        let d_acc = ristretto255::point_mul(d, &c);
        ristretto255::point_add_assign(&mut d_acc, &proof.x1);
        let g_alpha1 = ristretto255::basepoint_mul(&proof.alpha1);
        assert!(ristretto255::point_equals(&d_acc, &g_alpha1), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // c * y + X2 =? \alpha_2 * g
        let y_times_c = ristretto255::point_mul(&sender_pk_point, &c);
        ristretto255::point_add_assign(&mut y_times_c, &proof.x2);
        let g_alpha2 = ristretto255::basepoint_mul(&proof.alpha2);
        assert!(ristretto255::point_equals(&y_times_c, &g_alpha2), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        let g_alpha3 = ristretto255::basepoint_mul(&proof.alpha3);
        // c * C + X3 =? \alpha_3 * g + \alpha_1 * y
        let big_c_acc = ristretto255::point_mul(big_c, &c);
        ristretto255::point_add_assign(&mut big_c_acc, &proof.x3);
        let y_alpha1 = ristretto255::point_mul(&sender_pk_point, &proof.alpha1);
        ristretto255::point_add_assign(&mut y_alpha1, &g_alpha3);
        assert!(ristretto255::point_equals(&big_c_acc, &y_alpha1), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // c * \bar{C} + X4 =? \alpha_3 * g + \alpha_1 * \bar{y}
        let bar_c = ristretto255::point_mul(bar_c, &c);
        ristretto255::point_add_assign(&mut bar_c, &proof.x4);
        let bar_y_alpha1 = ristretto255::point_mul(&recipient_pk_point, &proof.alpha1);
        ristretto255::point_add_assign(&mut bar_y_alpha1, &g_alpha3);
        assert!(ristretto255::point_equals(&bar_c, &bar_y_alpha1), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));

        // c * (C_L + -C) + X5 =? \alpha_4 * g + \alpha_2 * (C_R + -D)
        let neg_C = ristretto255::point_neg(big_c);
        ristretto255::point_add_assign(&mut neg_C, c_L);
        ristretto255::point_mul_assign(&mut neg_C, &c);
        ristretto255::point_add_assign(&mut neg_C, &proof.x5);
        let neg_D = ristretto255::point_neg(d);
        ristretto255::point_add_assign(&mut neg_D, c_R);
        ristretto255::point_mul_assign(&mut neg_D, &proof.alpha2);
        let g_alpha4 = ristretto255::basepoint_mul(&proof.alpha4);
        ristretto255::point_add_assign(&mut neg_D, &g_alpha4);
        assert!(ristretto255::point_equals(&neg_C, &neg_D), error::invalid_argument(ESIGMA_PROTOCOL_VERIFY_FAILED));
    }

    /// Computes the challenge value as `c = H(g, y, \bar{y}, C_L, C_R, C, D, \bar{C}, X_1, X_2, X_3, X_4, X_5)`
    /// for the Sigma protocol from `verify_withdrawal_sigma_protocol` using the Fiat-Shamir transform. The notation
    /// used above is from the Zether [BAZB20] paper.
    fun sigma_protocol_fiat_shamir<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        balance_ct: &elgamal::Ciphertext,
        x1: &RistrettoPoint,
        x2: &RistrettoPoint,
        x3: &RistrettoPoint,
        x4: &RistrettoPoint,
        x5: &RistrettoPoint): Scalar
    {
        let (big_c, d) = elgamal::ciphertext_as_points(withdraw_ct);
        let (bar_c, _) = elgamal::ciphertext_as_points(deposit_ct);
        let (c_L, c_R) = elgamal::ciphertext_as_points(balance_ct);

        // c <- H(g,y,\bar{y},C_L,C_R,C,D,\bar{C},X_1,X_2,X_3,X_4,X_5)
        let hash_input = vector::empty<u8>();

        let basepoint_bytes = ristretto255::point_to_bytes(&ristretto255::basepoint_compressed());
        vector::append<u8>(&mut hash_input, basepoint_bytes);

        let y = elgamal::pubkey_to_compressed_point(sender_pk);
        let y_bytes = ristretto255::point_to_bytes(&y);
        vector::append<u8>(&mut hash_input, y_bytes);

        let y_bar = elgamal::pubkey_to_compressed_point(recipient_pk);
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

        let x_1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x1));
        vector::append<u8>(&mut hash_input, x_1_bytes);

        let x_2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x2));
        vector::append<u8>(&mut hash_input, x_2_bytes);

        let x_3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x3));
        vector::append<u8>(&mut hash_input, x_3_bytes);

        let x_4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x4));
        vector::append<u8>(&mut hash_input, x_4_bytes);

        let x_5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(x5));
        vector::append<u8>(&mut hash_input, x_5_bytes);

        vector::append<u8>(&mut hash_input, FIAT_SHAMIR_SIGMA_DST);

        ristretto255::new_scalar_from_sha2_512(hash_input)
    }

    //
    // Test-only functions
    //

    #[test_only]
    /// Returns a random ElGamal keypair
    fun generate_elgamal_keypair(): (Scalar, elgamal::CompressedPubkey) {
        let sk = ristretto255::random_scalar();
        let pk = elgamal::pubkey_from_secret_key(&sk);
        (sk, pk)
    }

    #[test_only]
    /// Returns true if the balance at address `owner` equals `value`.
    /// Requires the ElGamal encryption randomness `r` and public key `pk` as auxiliary inputs.
    public fun verify_opened_balance<CoinType>(
        owner: address, value: u64, r: &Scalar, pk: &elgamal::CompressedPubkey): bool acquires VeiledCoinStore
    {
        // compute the expected encrypted balance
        let value = ristretto255::new_scalar_from_u64(value);
        let expected_ct = elgamal::new_ciphertext_with_basepoint(&value, r, pk);

        // get the actual encrypted balance
        let actual_ct = elgamal::decompress_ciphertext(&veiled_balance<CoinType>(owner));

        elgamal::ciphertext_equals(&actual_ct, &expected_ct)
    }

    #[test_only]
    /// Proves the Sigma protocol used for veiled coin transfers.
    /// See `sigma_protocol_verify` for a detailed description of the sigma protocol
    public fun sigma_protocol_prove<CoinType>(
        sender_pk: &elgamal::CompressedPubkey,
        recipient_pk: &elgamal::CompressedPubkey,
        balance_ct: &elgamal::Ciphertext,
        withdraw_ct: &elgamal::Ciphertext,
        deposit_ct: &elgamal::Ciphertext,
        amount_rand: &Scalar,
        sk: &Scalar,
        amount_val: &Scalar,
        new_balance_val: &Scalar): SigmaProof<CoinType>
   {
        let x1 = ristretto255::random_scalar();
        let x2 = ristretto255::random_scalar();
        let x3 = ristretto255::random_scalar();
        let x4 = ristretto255::random_scalar();

        // X1 <- g^{x1}
        let big_x1 = ristretto255::basepoint_mul(&x1);

        // X2 <- g^{x2}
        let big_x2 = ristretto255::basepoint_mul(&x2);

        // X3 <- g^{x3}y^{x1}
        let big_x3 = ristretto255::basepoint_mul(&x3);
        let source_pk_point = elgamal::pubkey_to_point(sender_pk);
        let source_pk_x1 = ristretto255::point_mul(&source_pk_point, &x1);
        ristretto255::point_add_assign(&mut big_x3, &source_pk_x1);

        // X4 <- g^{x3}\bar{y}^{x1}
        let big_x4 = ristretto255::basepoint_mul(&x3);
        let dest_pk_point = elgamal::pubkey_to_point(recipient_pk);
        let dest_pk_x1 = ristretto255::point_mul(&dest_pk_point, &x1);
        ristretto255::point_add_assign(&mut big_x4, &dest_pk_x1);

        // X5 <- g^{x4}(C_R/D)^{x2}
        let big_x5 = ristretto255::basepoint_mul(&x4);
        let (_, c_R) = elgamal::ciphertext_as_points(balance_ct);
        let (_, big_d) = elgamal::ciphertext_as_points(withdraw_ct);
        let neg_d = ristretto255::point_neg(big_d);
        let c_R_acc = ristretto255::point_add(c_R, &neg_d);
        ristretto255::point_mul_assign(&mut c_R_acc, &x2);
        ristretto255::point_add_assign(&mut big_x5, &c_R_acc);

        let c = sigma_protocol_fiat_shamir<CoinType>(
            sender_pk, recipient_pk,
            withdraw_ct,
            deposit_ct,
            balance_ct,
            &big_x1, &big_x2, &big_x3, &big_x4, &big_x5);

        // alpha_1 <- x1 + c * r
        let alpha1 = ristretto255::scalar_mul(&c, amount_rand);
        ristretto255::scalar_add_assign(&mut alpha1, &x1);

        // alpha_2 <- x2 + c * sk
        let alpha2 = ristretto255::scalar_mul(&c, sk);
        ristretto255::scalar_add_assign(&mut alpha2, &x2);

        // alpha_3 <- x3 + c * b^*
        let alpha3 = ristretto255::scalar_mul(&c, amount_val);
        ristretto255::scalar_add_assign(&mut alpha3, &x3);

        // alpha_4 <- x4 + c * b'
        let alpha4 = ristretto255::scalar_mul(&c, new_balance_val);
        ristretto255::scalar_add_assign(&mut alpha4, &x4);

        SigmaProof {
            x1: big_x1,
            x2: big_x2,
            x3: big_x3,
            x4: big_x4,
            x5: big_x5,
            alpha1,
            alpha2,
            alpha3,
            alpha4
        }
    }

    #[test_only]
    /// Given a Sigma protocol proof, serializes it into byte form.
    /// Elements at the end of the `SigmaProof` struct are placed into the vector first,
    /// using the serialization formats in the `ristretto255` module.
    public fun serialize_sigma_proof<CoinType>(proof: &SigmaProof<CoinType>): vector<u8> {
        let x1_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x1));
        let x2_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x2));
        let x3_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x3));
        let x4_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x4));
        let x5_bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&proof.x5));
        let alpha1_bytes = ristretto255::scalar_to_bytes(&proof.alpha1);
        let alpha2_bytes = ristretto255::scalar_to_bytes(&proof.alpha2);
        let alpha3_bytes = ristretto255::scalar_to_bytes(&proof.alpha3);
        let alpha4_bytes = ristretto255::scalar_to_bytes(&proof.alpha4);

        let bytes = vector::empty<u8>();
        vector::append<u8>(&mut bytes, alpha4_bytes);
        vector::append<u8>(&mut bytes, alpha3_bytes);
        vector::append<u8>(&mut bytes, alpha2_bytes);
        vector::append<u8>(&mut bytes, alpha1_bytes);
        vector::append<u8>(&mut bytes, x5_bytes);
        vector::append<u8>(&mut bytes, x4_bytes);
        vector::append<u8>(&mut bytes, x3_bytes);
        vector::append<u8>(&mut bytes, x2_bytes);
        vector::append<u8>(&mut bytes, x1_bytes);

        bytes
    }

    //
    // Tests
    //

    #[test]
    fun sigma_proof_verify_test()
    {
        // Pick a keypair for the sender, and one for the recipient
        let (sender_sk, sender_pk) = generate_elgamal_keypair();
        let (_, recipient_pk) = generate_elgamal_keypair();

        // Set the sender's balance to 150 and encrypt it
        let balance_rand = ristretto255::random_scalar();
        let balance_val = ristretto255::new_scalar_from_u64(150);
        let balance_ct = elgamal::new_ciphertext_with_basepoint(&balance_val, &balance_rand, &sender_pk);

        // Set the transferred amount to 50
        let amount_val = ristretto255::new_scalar_from_u64(50);
        let amount_rand = ristretto255::random_scalar();
        // Encrypt the amount under the sender's PK
        let withdraw_ct= elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &sender_pk);

        // The remaining balance will be 150 - 50
        let new_balance_val = ristretto255::scalar_sub(&balance_val, &amount_val);

        // Encrypt the amount under the recipient's PK
        let deposit_ct = elgamal::new_ciphertext_with_basepoint(&amount_val, &amount_rand, &recipient_pk);

        let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(
            &sender_pk,
            &recipient_pk,
            &balance_ct,        // old balance, encrypted under sender PK
            &withdraw_ct,       // withdrawn amount, encrypted under sender PK
            &deposit_ct,        // deposited amount, encrypted under recipient PK (same plaintext as `withdraw_ct`)
            &amount_rand,       // encryption randomness for `withdraw_ct` and `deposit_ct`
            &sender_sk,
            &amount_val,        // transferred amount
            &new_balance_val,   // updated (plaintext) new balance of sender
        );

        sigma_protocol_verify(&sender_pk, &recipient_pk, &balance_ct, &withdraw_ct, &deposit_ct, &sigma_proof);
    }

    #[test]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    fun sigma_proof_verify_fails_test()
    {
       let (source_priv_key, source_pk) = generate_elgamal_keypair();
       let balance_rand = ristretto255::random_scalar();
       let balance_val = ristretto255::new_scalar_from_u64(150);
       let transfer_val = ristretto255::new_scalar_from_u64(50);
       let (_, dest_pk) = generate_elgamal_keypair();
       let balance_ct = elgamal::new_ciphertext_with_basepoint(&balance_val, &balance_rand, &source_pk);
       let transfer_rand = ristretto255::random_scalar();
       let (_, withdraw_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let new_balance_val = ristretto255::new_scalar_from_u64(100);
       let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &dest_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

       let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &balance_ct, &withdraw_ct, &deposit_ct, &transfer_rand, &source_priv_key, &transfer_val, &new_balance_val);

       let random_point = ristretto255::random_point();
       sigma_proof.x1 = random_point;

       sigma_protocol_verify(&source_pk, &dest_pk, &balance_ct, &withdraw_ct, &deposit_ct, &sigma_proof);
    }

    #[test]
    fun sigma_proof_serialize_test()
    {
       let (source_priv_key, source_pk) = generate_elgamal_keypair();
       let rand = ristretto255::random_scalar();
       let val = ristretto255::new_scalar_from_u64(50);
       let (_, dest_pk) = generate_elgamal_keypair();
       let balance_ct = elgamal::new_ciphertext_with_basepoint(&val, &rand, &source_pk);
       let source_randomness = ristretto255::scalar_neg(&rand);
       let (_, withdraw_ct) = bulletproofs::prove_range_elgamal(&val, &source_randomness, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
       let source_new_val = ristretto255::new_scalar_from_u64(100);
       let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&val, &rand, &dest_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

       let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &balance_ct, &withdraw_ct, &deposit_ct, &source_randomness, &source_priv_key, &val, &source_new_val);

       let sigma_proof_bytes = serialize_sigma_proof<coin::FakeMoney>(&sigma_proof);

       let deserialized_proof = std::option::extract<SigmaProof<coin::FakeMoney>>(&mut deserialize_sigma_proof<coin::FakeMoney>(sigma_proof_bytes));

       assert!(ristretto255::point_equals(&sigma_proof.x1, &deserialized_proof.x1), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x2, &deserialized_proof.x2), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x3, &deserialized_proof.x3), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x4, &deserialized_proof.x4), 1);
       assert!(ristretto255::point_equals(&sigma_proof.x5, &deserialized_proof.x5), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha1, &deserialized_proof.alpha1), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha2, &deserialized_proof.alpha2), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha3, &deserialized_proof.alpha3), 1);
       assert!(ristretto255::scalar_equals(&sigma_proof.alpha4, &deserialized_proof.alpha4), 1);
    }

    #[test(myself = @veiled_coin, source_fx = @aptos_framework, destination = @0x1337)]
    fun wrap_to_test(
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

        // Wrap 150 normal coins to veiled coins from the source's normal coin account
        // to the destination's veiled coin account
        let (_, destination_pk) = generate_elgamal_keypair();
        let destination_pk_bytes = elgamal::pubkey_to_bytes(&destination_pk);
        register<coin::FakeMoney>(&destination, destination_pk_bytes);
        veil_to<coin::FakeMoney>(&source_fx, destination_addr, 150);
        let source_balance = coin::balance<coin::FakeMoney>(source_addr);
        assert!(source_balance == 350, 1);
        let destination_rand = ristretto255::scalar_zero();
        assert!(verify_opened_balance<coin::FakeMoney>(destination_addr, 150, &destination_rand, &destination_pk), 1);

        // Unwrap back 50 veiled coins from the destination's veiled coin account to
        // the source's normal coin account
        let (_, source_pk) = generate_elgamal_keypair();
        let source_pk_bytes = elgamal::pubkey_to_bytes(&source_pk);
        register<coin::FakeMoney>(&source_fx, source_pk_bytes);

        let destination_new_balance = ristretto255::new_scalar_from_u64(100);

        let (new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(&destination_new_balance, &destination_rand, &destination_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
        let new_balance_range_proof_bytes = bulletproofs::range_proof_to_bytes(&new_balance_range_proof);

        unveil_to<coin::FakeMoney>(&destination, source_addr, 50, new_balance_range_proof_bytes);
        let source_balance = coin::balance<coin::FakeMoney>(source_addr);
        assert!(source_balance == 400, 1);
        assert!(verify_opened_balance<coin::FakeMoney>(destination_addr, 100, &destination_rand, &destination_pk), 1);
    }

    #[test(myself = @veiled_coin, source_fx = @aptos_framework, destination = @0x1337)]
    fun unwrap_test(
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

        // Create 500 fake money inside 'source'
        coin::create_fake_money(&source_fx, &destination, 500);

        // Mint 150 veiled coins at source (requires registering a veiled coin store at 'source')
        let (_, source_pk) = generate_elgamal_keypair();
        let source_pk_bytes = elgamal::pubkey_to_bytes(&source_pk);
        register<coin::FakeMoney>(&source_fx, source_pk_bytes);
        veil<coin::FakeMoney>(&source_fx, 150);

        // The unwrap function doesn't change the veiled coin account randomness,
        // so we use the zero scalar for it here
        let source_new_balance = ristretto255::new_scalar_from_u64(100);
        let new_balance_rand = ristretto255::scalar_zero();

        let (new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(&source_new_balance, &new_balance_rand, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
        unveil<coin::FakeMoney>(&source_fx, 50, bulletproofs::range_proof_to_bytes(&new_balance_range_proof));
        assert!(verify_opened_balance<coin::FakeMoney>(source_addr, 100, &new_balance_rand, &source_pk), 1);

        let nonveiled_balance = coin::balance<coin::FakeMoney>(source_addr);
        assert!(nonveiled_balance == 400, 1);
    }

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

        // Mint 150 veiled coins at source (requires registering a veiled coin store at 'source')
        let (source_priv_key, source_pk) = generate_elgamal_keypair();
        let source_pk_bytes = elgamal::pubkey_to_bytes(&source_pk);
        register<coin::FakeMoney>(&source_fx, source_pk_bytes);
        veil<coin::FakeMoney>(&source_fx, 150);

        let source_original_balance = veiled_balance<coin::FakeMoney>(source_addr);
        let source_original_balance = elgamal::decompress_ciphertext(&source_original_balance);
        // Transfer 50 of these veiled coins to destination
        let transfer_val = ristretto255::new_scalar_from_u64(50);
        let transfer_rand = ristretto255::random_scalar();

        // This will be the balance left at the source, that we need to do a range proof for
        let new_balance_rand_source = ristretto255::scalar_neg(&transfer_rand);
        let source_new_balance = ristretto255::new_scalar_from_u64(100);
        let (new_balance_range_proof, _) = bulletproofs::prove_range_elgamal(&source_new_balance, &new_balance_rand_source, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        let (transferred_amount_range_proof, withdraw_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        // Execute the veiled transaction: no one will be able to tell 50 coins are being transferred.
        let (_, dest_pk) = generate_elgamal_keypair();
        let dest_pk_bytes = elgamal::pubkey_to_bytes(&dest_pk);
        register<coin::FakeMoney>(&destination, dest_pk_bytes);

        let (_, deposit_ct) = bulletproofs::prove_range_elgamal(&transfer_val, &transfer_rand, &dest_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);

        let sigma_proof = sigma_protocol_prove<coin::FakeMoney>(&source_pk, &dest_pk, &source_original_balance, &withdraw_ct, &deposit_ct, &transfer_rand, &source_priv_key, &transfer_val, &source_new_balance);
        let sigma_proof_bytes = serialize_sigma_proof<coin::FakeMoney>(&sigma_proof);
        fully_veiled_transfer<coin::FakeMoney>(&source_fx, destination_addr, elgamal::ciphertext_to_bytes(&withdraw_ct), elgamal::ciphertext_to_bytes(&deposit_ct), bulletproofs::range_proof_to_bytes(&new_balance_range_proof), bulletproofs::range_proof_to_bytes(&transferred_amount_range_proof), sigma_proof_bytes);

        // Unwrap 25 coins from the source destination from veiled coins to regular coins
        let source_new_balance_unwrap = ristretto255::new_scalar_from_u64(75);

        // Unwrap doesn't change the randomness so we use the same randomness value as before
        let (new_balance_range_proof_unwrap, _) = bulletproofs::prove_range_elgamal(&source_new_balance_unwrap, &new_balance_rand_source, &source_pk, MAX_BITS_IN_VALUE, VEILED_COIN_DST);
        unveil<coin::FakeMoney>(&source_fx, 25, bulletproofs::range_proof_to_bytes(&new_balance_range_proof_unwrap));

        // Sanity check veiled balances
        assert!(verify_opened_balance<coin::FakeMoney>(source_addr, 75, &new_balance_rand_source, &source_pk), 1);
        assert!(verify_opened_balance<coin::FakeMoney>(destination_addr, 50, &transfer_rand, &dest_pk), 1);
    }
}
