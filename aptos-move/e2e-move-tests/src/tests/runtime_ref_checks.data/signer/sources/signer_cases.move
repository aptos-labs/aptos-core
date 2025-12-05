module runtime_ref_signer::cases {
    use std::signer;

    struct Marker has key, drop {
        dummy: bool,
    }

    public entry fun borrow_then_move(account: signer) {
        let addr_ref = signer::borrow_address(&account);
        let addr = *addr_ref;

        move_to(&account, Marker { dummy: true });
        let _ = move_from<Marker>(addr);
    }
}
