module token_lockup::token_lockup {
   use std::signer;
   use std::option;
   use std::error;
   use std::string::{Self, String};
   use std::object::{Self, Object, TransferRef, ConstructorRef};
   use std::timestamp;
   use velor_token_objects::royalty::{Royalty};
   use velor_token_objects::token::{Self, Token};
   use velor_token_objects::collection;

   #[resource_group_member(group = velor_framework::object::ObjectGroup)]
   struct LockupConfig has key {
      last_transfer: u64,
      transfer_ref: TransferRef,
   }

   /// The owner of the token has not owned it for long enough
   const ETOKEN_IN_LOCKUP: u64 = 0;
   /// The owner must own the token to transfer it
   const ENOT_TOKEN_OWNER: u64 = 1;

   const COLLECTION_NAME: vector<u8> = b"Rickety Raccoons";
   const COLLECTION_DESCRIPTION: vector<u8> = b"A collection of rickety raccoons!";
   const COLLECTION_URI: vector<u8> = b"https://ricketyracoonswebsite.com/collection/rickety-raccoon.png";
   const TOKEN_URI: vector<u8> = b"https://ricketyracoonswebsite.com/tokens/raccoon.png";
   const MAXIMUM_SUPPLY: u64 = 1000;
   // 24 hours in one day * 60 minutes in one hour * 60 seconds in one minute * 7 days
   const LOCKUP_PERIOD_SECS: u64 = (24 * 60 * 60) * 7;

   public fun initialize_collection(creator: &signer) {
      collection::create_fixed_collection(
         creator,
         string::utf8(COLLECTION_DESCRIPTION),
         MAXIMUM_SUPPLY,
         string::utf8(COLLECTION_NAME),
         option::none<Royalty>(),
         string::utf8(COLLECTION_URI),
      );
   }

   public fun mint_to(
      creator: &signer,
      token_name: String,
      to: address,
   ): ConstructorRef {
      let token_constructor_ref = token::create_named_token(
         creator,
         string::utf8(COLLECTION_NAME),
         string::utf8(COLLECTION_DESCRIPTION),
         token_name,
         option::none(),
         string::utf8(TOKEN_URI),
      );

      let transfer_ref = object::generate_transfer_ref(&token_constructor_ref);
      let token_signer = object::generate_signer(&token_constructor_ref);
      let token_object = object::object_from_constructor_ref<Token>(&token_constructor_ref);

      // transfer the token to the receiving account before we permanently disable ungated transfer
      object::transfer(creator, token_object, to);

      // disable the ability to transfer the token through any means other than the `transfer` function we define
      object::disable_ungated_transfer(&transfer_ref);

      move_to(
         &token_signer,
         LockupConfig {
            last_transfer: timestamp::now_seconds(),
            transfer_ref,
         }
      );

      token_constructor_ref
   }

   public entry fun transfer(
      from: &signer,
      token: Object<Token>,
      to: address,
   ) acquires LockupConfig {
      // redundant error checking for clear error message
      assert!(object::is_owner(token, signer::address_of(from)), error::permission_denied(ENOT_TOKEN_OWNER));
      let now = timestamp::now_seconds();
      let lockup_config = borrow_global_mut<LockupConfig>(object::object_address(&token));

      let time_since_transfer = now - lockup_config.last_transfer;
      let lockup_period_secs = LOCKUP_PERIOD_SECS;
      assert!(time_since_transfer >= lockup_period_secs, error::permission_denied(ETOKEN_IN_LOCKUP));

      // generate linear transfer ref and transfer the token object
      let linear_transfer_ref = object::generate_linear_transfer_ref(&lockup_config.transfer_ref);
      object::transfer_with_ref(linear_transfer_ref, to);

      // update the lockup config to reflect the latest transfer time
      *&mut lockup_config.last_transfer = now;
   }

   #[view]
   public fun view_last_transfer(
      token: Object<Token>,
   ): u64 acquires LockupConfig {
      borrow_global<LockupConfig>(object::object_address(&token)).last_transfer
   }
}
