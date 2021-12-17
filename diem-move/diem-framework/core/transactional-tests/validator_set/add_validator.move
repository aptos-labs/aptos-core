//# init --validators Bob
//#      --addresses Alex=0x4b7653f6566a52c9b496f245628a69a0
//#      --private-keys Alex=f5fd1521bd82454a9834ef977c389a0201f9525b11520334842ab73d2dcbf8b7

// Add simple validator to DiemSystem's validator set.

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemSystem;
    use DiemFramework::ValidatorConfig;
    fun main() {
        // test bob is a validator
        assert!(ValidatorConfig::is_valid(@Bob) == true, 98);
        assert!(DiemSystem::is_validator(@Bob) == true, 98);
    }
}

//# run --signers DiemRoot --args 0 0xAA x"00000000000000000000000000000000" b"owner_name"
//#     -- 0x1::AccountCreationScripts::create_validator_account

//# run --signers DiemRoot --args 0 @Alex x"042771c10e14f6436838e2c88a19873d" b"alex"
//#     -- 0x1::AccountCreationScripts::create_validator_account

// TODO(valerini): enable the following test once the sender format is supported
// //! new-transaction
// //! sender: 0xAA
// script {
// fun main() {
//
//     // add itself as a validator
// }
// }
//
// // check: "Keep(EXECUTED)"
// // check: NewEpochEvent
