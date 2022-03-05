//# init --parent-vasps Parent1 Parent2
//#      --addresses Child1=0xe42bd8dd8e9a3c5cdcb0a99619884fa1
//#                  Child2=0xdd00316615da8ef1b1114c6a9f20cd8a
//#      --private-keys Child1=915d621309cf25b9ae00c7ea7d2a2e99dcd77a2b71b79a5cf44c0291d8bdce6f
//#                     Child2=75f1fbb7f1bc78e9de643ee11589c92a53ee616de15cdad0cc48f89024b55a4b



// === Setup ===

//# run --signers Parent1
//#     --type-args 0x1::XUS::XUS
//#     --args @Child1
//#            x"e59531e507f309f5731a67600c845078"
//#            false
//#            0
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account

//# run --signers Parent1
//#     --type-args 0x1::XUS::XUS
//#     --args @Child2
//#            x"189804b7934e02fd0e57e66977819e81"
//#            false
//#            0
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account



// === Intended usage ===

// Make child1 a recovery address.
//
//# run --signers Child1 -- 0x1::AccountAdministrationScripts::create_recovery_address

// Delegate parent1's key to child1.
//
//# run --signers Parent1 --args @Child1 -- 0x1::AccountAdministrationScripts::add_recovery_rotation_capability



// ==== Abort cases ===

// Delegating parent2's key to child1 should abort because they are different VASPs.
//
//# run --signers Parent2 --args @Child1 -- 0x1::AccountAdministrationScripts::add_recovery_rotation_capability

// Delegating parent2's key to an account without a RecoveryAddress resource should abort.
//
//# run --signers Parent2 --args 0x3333 -- 0x1::AccountAdministrationScripts::add_recovery_rotation_capability

// Trying to recover an account that hasn't delegated its KeyRotationCapability to a recovery.
//
//# run --signers Child2
//#     --args @Child1 @Child2 x"7013b6ed7dde3cfb1251db1b04ae9cd7853470284085693590a75def645a926d"
//#     -- 0x1::AccountAdministrationScripts::rotate_authentication_key_with_recovery_address

// Trying to recover from an account without a RecoveryAddress resource should abort.
//
//# run --signers Child1
//#     --args @Child2 @Child1 x"7013b6ed7dde3cfb1251db1b04ae9cd7853470284085693590a75def645a926d"
//#     -- 0x1::AccountAdministrationScripts::rotate_authentication_key_with_recovery_address

// Parent1 shouldn't be able to rotate child1's address.
//
//# run --signers Parent1
//#     --args @Child1 @Child1 x"7013b6ed7dde3cfb1251db1b04ae9cd7853470284085693590a75def645a926d"
//#     -- 0x1::AccountAdministrationScripts::rotate_authentication_key_with_recovery_address

// A non-vasp can't create a recovery address
//
//# run --signers TreasuryCompliance -- 0x1::AccountAdministrationScripts::create_recovery_address
