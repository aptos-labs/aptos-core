//# init --parent-vasps Alice Test

// TODO: consider converting some of these into unit tests.

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;
    // Make sure that XUS is registered. Make sure that the rules
    // relating to SCS and synthetic currencies are consistent
    fun main() {
        assert!(Diem::is_currency<XUS>(), 1);
        assert!(!Diem::is_synthetic_currency<XUS>(), 2);
        assert!(Diem::is_SCS_currency<XUS>(), 4);
        Diem::assert_is_currency<XUS>();
        Diem::assert_is_SCS_currency<XUS>();
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use DiemFramework::XDX::XDX;

    fun main() {
        Diem::assert_is_SCS_currency<XDX>();
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;

    fun main() {
        Diem::assert_is_currency<u64>();
    }
}

//# run --signers Alice
//#     --type-args 0x1::XUS::XUS
//#     --args 0 1 3
//#     -- 0x1::TreasuryComplianceScripts::update_exchange_rate

//# run --signers Alice
//#     --type-args 0x1::XUS::XUS
//#     --args false
//#     -- 0x1::TreasuryComplianceScripts::update_minting_ability

//# publish
module Test::Holder {
    struct Holder<T> has key { x: T }
    public fun hold<T: store>(account: &signer, x: T) {
        move_to(account, Holder<T>{ x })
    }

    public fun get<T: store>(addr: address): T
    acquires Holder {
       let Holder<T> { x } = move_from<Holder<T>>(addr);
       x
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;
    use Std::FixedPoint32;
    use Test::Holder;

    fun main(dr: signer, _dr2: signer) {
        let (a, b) = Diem::register_currency<XUS>(
            &dr,
            FixedPoint32::create_from_rational(1, 1),
            false,
            1000,
            10,
            b"ABC",
        );

        Holder::hold(&dr, a);
        Holder::hold(&dr, b);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use Std::FixedPoint32;
    use Test::Holder;

    fun main(dr: signer, _dr2: signer) {
        let (a, b) = Diem::register_currency<u64>(
            &dr,
            FixedPoint32::create_from_rational(1,1),
            false,
            0, // scaling factor
            100,
            x""
        );
        Holder::hold(&dr, a);
        Holder::hold(&dr, b);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use Std::FixedPoint32;
    use Test::Holder;

    fun main(dr: signer, _dr2: signer) {
        let (a, b) = Diem::register_currency<u64>(
            &dr,
            FixedPoint32::create_from_rational(1,1),
            false,
            1000000000000000, // scaling factor > MAX_SCALING_FACTOR
            100,
            x""
        );
        Holder::hold(&dr, a);
        Holder::hold(&dr, b);
    }
}
