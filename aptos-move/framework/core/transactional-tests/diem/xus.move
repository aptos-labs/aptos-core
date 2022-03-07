//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(Diem::approx_xdx_for_value<XUS>(10) == 10, 1);
        assert!(Diem::scaling_factor<XUS>() == 1000000, 2);
        assert!(Diem::fractional_part<XUS>() == 100, 3);
    }
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 1 3
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::update_exchange_rate

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;

    fun main() {
        assert!(Diem::approx_xdx_for_value<XUS>(10) == 3, 4);
    }
}
