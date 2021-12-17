//# init

// --------------------------------------------------------------------
// BLESSED treasury compliant account initiate first tier
//
//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 0xDEADBEEF x"00000000000000000000000000000001" x"" false
//#     --show-events
//#     -- 0x1::AccountCreationScripts::create_designated_dealer

// --------------------------------------------------------------------
// Blessed treasury initiate mint flow given DD creation
// Test add and update tier functions
//
//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 0xDEADBEEF 1000000 0
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint

//TODO(moezinia) add burn txn once specific address directive sender complete
// and with new burn flow
