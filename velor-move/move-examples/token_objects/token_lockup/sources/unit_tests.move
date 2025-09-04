#[test_only]
module token_lockup::unit_tests {
   use velor_framework::object;
   use velor_framework::account;
   use velor_framework::timestamp;
   use velor_token_objects::token::{Token};
   use std::signer;
   use std::string::{Self};
   use token_lockup::token_lockup;

   const TEST_START_TIME: u64 = 1000000000;
   // 24 hours in one day * 60 minutes in one hour * 60 seconds in one minute * 7 days
   const LOCKUP_PERIOD_SECS: u64 = (24 * 60 * 60) * 7;

   fun setup_test(
       creator: &signer,
       owner_1: &signer,
       owner_2: &signer,
       velor_framework: &signer,
       start_time: u64,
   ) {
         timestamp::set_time_has_started_for_testing(velor_framework);
         timestamp::update_global_time_for_test_secs(start_time);
         account::create_account_for_test(signer::address_of(creator));
         account::create_account_for_test(signer::address_of(owner_1));
         account::create_account_for_test(signer::address_of(owner_2));
         token_lockup::initialize_collection(creator);
   }

   fun fast_forward_secs(seconds: u64) {
      timestamp::update_global_time_for_test_secs(timestamp::now_seconds() + seconds);
   }

   #[test(creator = @0xFA, owner_1 = @0xA, owner_2 = @0xB, velor_framework = @0x1)]
   /// Tests transferring multiple tokens to different owners with slightly different initial lockup times
   fun test_happy_path(
      creator: &signer,
      owner_1: &signer,
      owner_2: &signer,
      velor_framework: &signer,
   ) {
      setup_test(creator, owner_1, owner_2, velor_framework, TEST_START_TIME);

      let owner_1_addr = signer::address_of(owner_1);
      let owner_2_addr = signer::address_of(owner_2);

      // mint 1 token to each of the 2 owner accounts
      let token_1_constructor_ref = token_lockup::mint_to(creator, string::utf8(b"Token #1"), owner_1_addr);
      let token_2_constructor_ref = token_lockup::mint_to(creator, string::utf8(b"Token #2"), owner_2_addr);
      // mint 1 more token to owner_1 one second later
      fast_forward_secs(1);
      let token_3_constructor_ref = token_lockup::mint_to(creator, string::utf8(b"Token #3"), owner_1_addr);

      let token_1_obj = object::object_from_constructor_ref(&token_1_constructor_ref);
      let token_2_obj = object::object_from_constructor_ref(&token_2_constructor_ref);
      let token_3_obj = object::object_from_constructor_ref(&token_3_constructor_ref);

      // fast forward global time by 1 week - 1 second
      fast_forward_secs((LOCKUP_PERIOD_SECS) - 1);

      // ensures that the `last_transfer` for each token is correct
      assert!(token_lockup::view_last_transfer(token_1_obj) == TEST_START_TIME, 0);
      assert!(token_lockup::view_last_transfer(token_2_obj) == TEST_START_TIME, 1);
      assert!(token_lockup::view_last_transfer(token_3_obj) == TEST_START_TIME + 1, 2);

      // transfer the first token from owner_1 to owner_2
      token_lockup::transfer(owner_1, token_1_obj, owner_2_addr);
      // transfer the second token from owner_2 to owner_1
      token_lockup::transfer(owner_2, token_2_obj, owner_1_addr);
      // fast forward global time by 1 second
      fast_forward_secs(1);
      // transfer the third token from owner_1 to owner_2
      token_lockup::transfer(owner_1, token_3_obj, owner_2_addr);
      // ensures that the `last_transfer` for each token is correct
      assert!(token_lockup::view_last_transfer(token_1_obj) == TEST_START_TIME + (LOCKUP_PERIOD_SECS), 3);
      assert!(token_lockup::view_last_transfer(token_2_obj) == TEST_START_TIME + (LOCKUP_PERIOD_SECS), 4);
      assert!(token_lockup::view_last_transfer(token_3_obj) == TEST_START_TIME + (LOCKUP_PERIOD_SECS) + 1, 5);

      // ensures that the owners respectively are owner_2, owner_1, and owner_2
      assert!(object::is_owner(token_1_obj, owner_2_addr), 6);
      assert!(object::is_owner(token_2_obj, owner_1_addr), 7);
      assert!(object::is_owner(token_3_obj, owner_2_addr), 8);
   }

   #[test(creator = @0xFA, owner_1 = @0xA, owner_2 = @0xB, velor_framework = @0x1)]
   #[expected_failure(abort_code = 0x50003, location = velor_framework::object)]
   fun transfer_raw_fail(
      creator: &signer,
      owner_1: &signer,
      owner_2: &signer,
      velor_framework: &signer,
   ) {
      setup_test(creator, owner_1, owner_2, velor_framework, TEST_START_TIME);

      let token_1_constructor_ref = token_lockup::mint_to(creator, string::utf8(b"Token #1"), signer::address_of(owner_1));
      object::transfer_raw(
         owner_1,
         object::address_from_constructor_ref(&token_1_constructor_ref),
         signer::address_of(owner_2)
      );
   }

   #[test(creator = @0xFA, owner_1 = @0xA, owner_2 = @0xB, velor_framework = @0x1)]
   #[expected_failure(abort_code = 0x50000, location = token_lockup::token_lockup)]
   fun transfer_too_early(
      creator: &signer,
      owner_1: &signer,
      owner_2: &signer,
      velor_framework: &signer,
   ) {
      setup_test(creator, owner_1, owner_2, velor_framework, TEST_START_TIME);

      let token_1_constructor_ref = token_lockup::mint_to(creator, string::utf8(b"Token #1"), signer::address_of(owner_1));
      let token_1_obj = object::object_from_constructor_ref(&token_1_constructor_ref);

      // one second too early
      fast_forward_secs((LOCKUP_PERIOD_SECS) - 1);
      token_lockup::transfer(owner_1, token_1_obj, signer::address_of(owner_2));
   }

   #[test(creator = @0xFA, owner_1 = @0xA, owner_2 = @0xB, velor_framework = @0x1)]
   #[expected_failure(abort_code = 0x50001, location = token_lockup::token_lockup)]
   fun transfer_wrong_owner(
      creator: &signer,
      owner_1: &signer,
      owner_2: &signer,
      velor_framework: &signer,
   ) {
      setup_test(creator, owner_1, owner_2, velor_framework, TEST_START_TIME);

      let token_1_constructor_ref = token_lockup::mint_to(creator, string::utf8(b"Token #1"), signer::address_of(owner_1));
      let token_1_obj = object::object_from_constructor_ref<Token>(&token_1_constructor_ref);

      fast_forward_secs(LOCKUP_PERIOD_SECS);
      token_lockup::transfer(owner_2, token_1_obj, signer::address_of(owner_1));
   }
}
