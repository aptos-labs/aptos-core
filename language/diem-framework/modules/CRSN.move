/// A module implementing conflict-resistant sequence numbers (CRSNs).
/// The specification, and formal description of the acceptance and rejection
/// criteria, force expiration and window shifting of CRSNs are described in DIP-168.

module DiemFramework::CRSN {
    use Std::BitVector::{Self, BitVector};
    use Std::Signer;
    use Std::Errors;

    friend DiemFramework::DiemAccount;

    /// A CRSN  represents a finite slice or window of an "infinite" bitvector
    /// starting at zero with a size `k` defined dynamically at the time of
    /// publication of CRSN resource. The `min_nonce` defines the left-hand
    /// side of the slice, and the slice's state is held in `slots` and is of size `k`.
    /// Diagrammatically:
    /// ```
    /// 1111...000000100001000000...0100001000000...0000...
    ///        ^             ...                ^
    ///        |____..._____slots______...______|
    ///     min_nonce                       min_nonce + k - 1
    /// ```
    struct CRSN has key {
        min_nonce: u64,
        size: u64,
        slots: BitVector,
    }

    /// No CRSN resource exists
    const ENO_CRSN: u64 = 0;
    /// A CRSN resource wasn't expected, but one was found
    const EHAS_CRSN: u64 = 1;
    /// The size given to the CRSN at the time of publishing was zero, which is not supported
    const EZERO_SIZE_CRSN: u64 = 2;
    /// The size given to the CRSN at the time of publishing was larger than the largest allowed CRSN size
    const ECRSN_SIZE_TOO_LARGE: u64 = 3;
    /// the amount to shift the CRSN window was zero
    const EINVALID_SHIFT: u64 = 4;

    const MAX_CRSN_SIZE: u64 = 256;


    /// Publish a DSN under `account`. Cannot already have a DSN published.
    public(friend) fun publish(account: &signer, min_nonce: u64, size: u64) {
        assert(!has_crsn(Signer::address_of(account)), Errors::invalid_state(EHAS_CRSN));
        assert(size > 0, Errors::invalid_argument(EZERO_SIZE_CRSN));
        assert(size <= MAX_CRSN_SIZE, Errors::invalid_argument(ECRSN_SIZE_TOO_LARGE));
        move_to(account, CRSN {
            min_nonce,
            size,
            slots: BitVector::new(size),
        })
    }
    spec publish {
        include BitVector::NewAbortsIf{length: size};
        aborts_if has_crsn(Signer::spec_address_of(account)) with Errors::INVALID_STATE;
        aborts_if size == 0 with Errors::INVALID_ARGUMENT;
        aborts_if size > MAX_CRSN_SIZE with Errors::INVALID_ARGUMENT;
        ensures exists<CRSN>(Signer::spec_address_of(account));
    }

    /// Record `sequence_nonce` under the `account`. Returns true if
    /// `sequence_nonce` is accepted, returns false if the `sequence_nonce` is rejected.
    public(friend) fun record(account: &signer, sequence_nonce: u64): bool
    acquires CRSN {
        let addr = Signer::address_of(account);
        if (check(account, sequence_nonce)) {
            // CRSN exists by `check`.
            let crsn = borrow_global_mut<CRSN>(addr);
            // accept nonce
            let scaled_nonce = sequence_nonce - crsn.min_nonce;
            BitVector::set(&mut crsn.slots, scaled_nonce);
            shift_window_right(crsn);
            return true
        } else if (exists<CRSN>(addr)) { // window was force shifted in this transaction
            let crsn = borrow_global<CRSN>(addr);
            if (crsn.min_nonce > sequence_nonce) return true
        };

        false
    }

    /// A stateless version of `record`: returns `true` if the `sequence_nonce`
    /// will be accepted, and `false` otherwise.
    public(friend) fun check(account: &signer, sequence_nonce: u64): bool
    acquires CRSN {
        let addr = Signer::address_of(account);
        assert(has_crsn(addr), Errors::invalid_state(ENO_CRSN));
        let crsn = borrow_global_mut<CRSN>(addr);

        // Don't accept if it's outside of the window
        if ((sequence_nonce < crsn.min_nonce) ||
            ((sequence_nonce as u128) >= (crsn.min_nonce as u128) + (BitVector::length(&crsn.slots) as u128))) {
            false
        } else {
            // scaled nonce is the index in the window
            let scaled_nonce = sequence_nonce - crsn.min_nonce;

            // Bit already set, reject, otherwise accept
            !BitVector::is_index_set(&crsn.slots, scaled_nonce)
        }
    }
    spec check {
        include CheckAbortsIf{addr: Signer::spec_address_of(account)};
    }
    spec schema CheckAbortsIf {
        addr: address;
        sequence_nonce: u64;
        let crsn = global<CRSN>(addr);
        let scaled_nonce = sequence_nonce - crsn.min_nonce;
        aborts_if !has_crsn(addr) with Errors::INVALID_STATE;
        include has_crsn(addr) &&
                (sequence_nonce >= crsn.min_nonce) &&
                (sequence_nonce + crsn.min_nonce < BitVector::length(crsn.slots))
        ==> BitVector::IsIndexSetAbortsIf{bitvector: crsn.slots, bit_index: scaled_nonce };
    }
    spec fun spec_check(addr: address, sequence_nonce: u64): bool {
        let crsn = global<CRSN>(addr);
        if ((sequence_nonce < crsn.min_nonce) ||
            (sequence_nonce >= crsn.min_nonce + BitVector::length(crsn.slots))) {
            false
        } else {
            let scaled_nonce = sequence_nonce - crsn.min_nonce;
            !BitVector::spec_is_index_set(crsn.slots, scaled_nonce)
        }
    }

    /// Force expire transactions by forcibly shifting the window by
    /// `shift_amount`. After the window has been shifted by `shift_amount` it is
    /// then shifted over set bits as define by the `shift_window_right` function.
    public fun force_expire(account: &signer, shift_amount: u64)
    acquires CRSN {
        assert(shift_amount > 0, Errors::invalid_argument(EINVALID_SHIFT));
        let addr = Signer::address_of(account);
        assert(has_crsn(addr), Errors::invalid_state(ENO_CRSN));
        let crsn = borrow_global_mut<CRSN>(addr);

        BitVector::shift_left(&mut crsn.slots, shift_amount);

        crsn.min_nonce = crsn.min_nonce + shift_amount;
        // shift over any set bits
        shift_window_right(crsn);
    }

    /// Return whether this address has a CRSN resource published under it.
    public fun has_crsn(addr: address): bool {
        exists<CRSN>(addr)
    }

    fun shift_window_right(crsn: &mut CRSN) {
        let index = BitVector::longest_set_sequence_starting_at(&crsn.slots, 0);

        // if there is no run of set bits return early
        if (index == 0) return;
        BitVector::shift_left(&mut crsn.slots, index);
        crsn.min_nonce = crsn.min_nonce + index;
    }


    /***************************************************************************/
    // tests
    /***************************************************************************/

    #[test_only]
    public fun test_publish(account: &signer, min_nonce: u64, size: u64) {
        publish(account, min_nonce, size)
    }

    #[test_only]
    public fun test_record(account: &signer, sequence_nonce: u64): bool
    acquires CRSN {
        record(account, sequence_nonce)
    }

    #[test_only]
    public fun test_check(account: &signer, sequence_nonce: u64): bool
    acquires CRSN {
        check(account, sequence_nonce)
    }

    #[test_only]
    public fun test_force_expire(account: &signer, shift_amount: u64)
    acquires CRSN {
        force_expire(account, shift_amount)
    }

    #[test_only]
    public fun slots(account: address): BitVector
    acquires CRSN {
        *&borrow_global<CRSN>(account).slots
    }

    #[test_only]
    public fun min_nonce(account: address): u64
    acquires CRSN {
        *&borrow_global<CRSN>(account).min_nonce
    }

    #[test_only]
    public fun size(account: address): u64
    acquires CRSN {
        *&borrow_global<CRSN>(account).size
    }

    #[test_only]
    public fun max_crsn_size(): u64 {
        MAX_CRSN_SIZE
    }
}
