//# init --parent-vasps Ricky

// --------------------------------------------------------------------
// BLESSED treasury compliance acccount creates DD with tiers of one coin type
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
//#     --args 0 0xDEADBEEF 99000000 0
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint

// --------------------------------------------------------------------
// Mint initiated
//
//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 0xDEADBEEF 5000001000000 0
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint

// --------------------------------------------------------------------
// Validate regular account can not initiate mint, only Blessed treasury account
//
//# run --signers Ricky
//#     --type-args 0x1::XUS::XUS
//#     --args 0 0xDEADBEEF 1 0
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint
