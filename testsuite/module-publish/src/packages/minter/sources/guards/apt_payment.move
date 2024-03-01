module token_minter::apt_payment {

    use std::error;
    use std::signer;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::object;
    use aptos_framework::object::Object;

    friend token_minter::token_minter;

    /// AptPayment object does not exist at the given address.
    const EAPT_PAYMENT_DOES_NOT_EXIST: u64 = 1;
    /// Insufficient payment for the given amount.
    const EINSUFFICIENT_PAYMENT: u64 = 2;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct AptPayment has key {
        amount: u64,
        destination: address,
    }

    public(friend) fun add_or_update_apt_payment<T: key>(
        token_minter_signer: &signer,
        token_minter: Object<T>,
        amount: u64,
        destination: address,
    ) acquires AptPayment {
        if (is_apt_payment_enabled(token_minter)) {
            let apt_payment = borrow_mut<T>(token_minter);
            apt_payment.amount = amount;
            apt_payment.destination = destination;
        } else {
            move_to(token_minter_signer, AptPayment { amount, destination });
        }
    }

    public(friend) fun remove_apt_payment<T: key>(token_minter: Object<T>) acquires AptPayment {
        let token_minter_address = apt_payment_address(token_minter);
        let AptPayment { amount: _, destination: _ } = move_from<AptPayment>(token_minter_address);
    }

    public(friend) fun execute<T: key>(
        minter: &signer,
        token_minter: Object<T>,
        amount: u64,
    ) acquires AptPayment {
        let apt_payment = borrow<T>(token_minter);
        let total_cost = apt_payment.amount * amount;
        assert!(
            coin::balance<AptosCoin>(signer::address_of(minter)) >= total_cost,
            error::invalid_state(EINSUFFICIENT_PAYMENT),
        );

        coin::transfer<AptosCoin>(minter, apt_payment.destination, total_cost);
    }

    inline fun borrow<T: key>(token_minter: Object<T>): &AptPayment acquires AptPayment {
        borrow_global<AptPayment>(apt_payment_address(token_minter))
    }

    inline fun borrow_mut<T: key>(token_minter: Object<T>): &mut AptPayment acquires AptPayment {
        borrow_global_mut<AptPayment>(apt_payment_address(token_minter))
    }

    fun apt_payment_address<T: key>(token_minter: Object<T>): address {
        let apt_payment_address = object::object_address(&token_minter);
        assert!(is_apt_payment_enabled(token_minter), error::not_found(EAPT_PAYMENT_DOES_NOT_EXIST));

        apt_payment_address
    }

    // ================================== View functions ================================== //

    #[view]
    public fun is_apt_payment_enabled<T: key>(token_minter: Object<T>): bool {
        exists<AptPayment>(object::object_address(&token_minter))
    }

    #[view]
    public fun amount<T: key>(token_minter: Object<T>): u64 acquires AptPayment {
        borrow(token_minter).amount
    }

    #[view]
    public fun destination<T: key>(token_minter: Object<T>): address acquires AptPayment {
        borrow(token_minter).destination
    }
}
