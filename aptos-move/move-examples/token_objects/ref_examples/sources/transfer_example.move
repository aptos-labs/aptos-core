module ref_examples::transfer_example {
   use std::signer;
   use std::option;
   use std::error;
   use std::string::{Self, String};
   use std::object::{Self, Object, TransferRef};
   use aptos_token_objects::royalty::{Royalty};
   use aptos_token_objects::token::{Self, Token};
   use aptos_token_objects::collection;

   #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
   struct Refs has key {
      transfer_ref: TransferRef,
   }

   /// The account that calls this function must be the module's designated admin, as set in the `Move.toml` file.
   const ENOT_ADMIN: u64 = 0;

   // Collection configuration details
   const COLLECTION_NAME: vector<u8> = b"Ornery Owls";
   const COLLECTION_DESCRIPTION: vector<u8> = b"A collection of ornery owls";
   const COLLECTION_URI: vector<u8> = b"your_collection_image_uri_here";
   const TOKEN_URI: vector<u8> = b"your_token_image_uri_here";
   const MAXIMUM_SUPPLY: u64 = 10;

   /// Ensure that the deployer is the @admin of the module, then create the collection.
   /// Note that `init_module` is automatically run when the contract is published.
   fun init_module(deployer: &signer) {
        assert!(signer::address_of(deployer) == @admin, error::permission_denied(ENOT_ADMIN));
        create_collection(deployer);
   }

   /// This function handles creating the token, minting it to the specified `to` address,
   /// and storing the `TransferRef` for the Token in its `Refs` resource.
   /// This means every time we create a new Token, we create and move a Refs resource
   /// to its global address. This is how we can keep track of the TransferRef for each
   /// individual Token we create.
   /// @returns the address of the newly created Token Object
   public fun mint_to(
      admin: &signer,
      token_name: String,
      to: address,
   ): address {
      // create the token and get back the &ConstructorRef to create the other Refs with
      let token_constructor_ref = token::create_named_token(
         admin,
         string::utf8(COLLECTION_NAME),
         string::utf8(COLLECTION_DESCRIPTION),
         token_name,
         option::none(),
         string::utf8(TOKEN_URI),
      );

      // create the TransferRef, the token's `&signer`, and the token's `&Object`
      let transfer_ref = object::generate_transfer_ref(&token_constructor_ref);
      let token_signer = object::generate_signer(&token_constructor_ref);
      let token_object = object::object_from_constructor_ref<Token>(&token_constructor_ref);

      // transfer the token to the receiving account
      object::transfer(admin, token_object, to);

      // create the Refs resource with the TransferRef we generated
      let refs = Refs {
        transfer_ref,
      };

      // Move the Refs resource to the Token's global resources
      move_to(
         &token_signer,
         refs,
      );

      signer::address_of(&token_signer)
   }

    /// This function requires elevated admin access, as it handles transferring the token
    /// to the specified `to` address regardless of who owns it.
   public entry fun transfer(
      admin: &signer,
      token: Object<Token>,
      to: address,
   ) acquires Refs {
      // Ensure that the caller is the @admin of the module
      assert!(signer::address_of(admin) == @admin, error::permission_denied(ENOT_ADMIN));

      // In order to call `object::transfer_with_ref`, we must possess a `LinearTransferRef`,
      // which gives us the right to a one-time unilateral transfer, regardless of the Object's owner.

      // 1. First, we must borrow the `Refs` resource at the token's address, which contains the `TransferRef`
      let refs = borrow_global<Refs>(object::object_address(&token));

      // 2. Generate the linear transfer ref with a reference to the Token's `Ref.transfer_ref: TransferRef`
      let linear_transfer_ref = object::generate_linear_transfer_ref(&refs.transfer_ref);

      // 3. Transfer the token to the receiving `to` account
      object::transfer_with_ref(linear_transfer_ref, to);
   }

   /// Helper function to create the collection
   public fun create_collection(admin: &signer) {
      collection::create_fixed_collection(
         admin,
         string::utf8(COLLECTION_DESCRIPTION),
         MAXIMUM_SUPPLY,
         string::utf8(COLLECTION_NAME),
         option::none<Royalty>(),
         string::utf8(COLLECTION_URI),
      );
   }

   //            //
   // Unit tests //
   //            //

   #[test_only]
   use ref_examples::transfer_example::{Self};

   #[test_only]
   /// Helper function to initialize the test and create and return the three admin/owner accounts
   fun init_for_test(
      admin: &signer,
   ) {
      // Normally we might put some more complex logic in here if we regularly instantiate multiple
      // accounts and logistical things for each test

      // For this, we just need to call the initialization function by directly invoking it.
      // It would normally be automatically called upon publishing the module, but since this
      // is a unit test, we have to manually call it.
      transfer_example::init_module(admin);
   }

   #[test(admin = @admin, owner_1 = @0xA, owner_2 = @0xB)]
   /// Tests creating a token and transferring it to multiple owners
   fun test_happy_path(
      admin: &signer,
      owner_1: &signer,
      owner_2: &signer,
   ) acquires Refs {
      init_for_test(admin);
      let admin_address = signer::address_of(admin);
      let owner_1_address = signer::address_of(owner_1);
      let owner_2_address = signer::address_of(owner_2);

      // Admin is now the owner of the collection, so let's mint a token to owner_1
      let token_address = transfer_example::mint_to(admin, string::utf8(b"Token #1"), owner_1_address);
      let token_object = object::address_to_object<Token>(token_address);

      assert!(object::is_owner(token_object, owner_1_address), 0);

      // Now let's transfer the token to owner_2, without owner_2's permission.
      transfer_example::transfer(admin, token_object, owner_2_address);
      assert!(object::is_owner(token_object, owner_2_address), 0);

      // Now let's transfer the token back to admin, without owner_2's permission.
      transfer_example::transfer(admin, token_object, admin_address);
      assert!(object::is_owner(token_object, admin_address), 0);
   }

   // Test to ensure the deployer must set to admin
   // see error.move for more error codes
   // PERMISSION_DENIED = 0x5; // turns into 0x50000 when emitted from error.move
   // ENOT_ADMIN = 0x0;
   // thus expected_failure = PERMISSION_DENIED + ENOT_ADMIN = 0x50000 + 0x0 = 0x50000
   #[test(admin = @0xFA)]
   #[expected_failure(abort_code = 0x50000, location = Self)]
   /// Tests creating a token and transferring it to multiple owners
   fun test_not_admin_for_init(
      admin: &signer,
   ) {
      transfer_example::init_module(admin);
   }

   // Test to ensure that the only account that can call `transfer` is the module's admin
   #[test(admin = @admin, owner_1 = @0xA, owner_2 = @0xB)]
   #[expected_failure(abort_code = 0x50000, location = Self)]
   fun test_not_admin_for_transfer(
      admin: &signer,
      owner_1: &signer,
      owner_2: &signer,
   ) acquires Refs {
      init_for_test(admin);
      let owner_1_address = signer::address_of(owner_1);
      let owner_2_address = signer::address_of(owner_2);
      let token_address = transfer_example::mint_to(admin, string::utf8(b"Token #1"), owner_1_address);
      let token_object = object::address_to_object<Token>(token_address);
      assert!(object::is_owner(token_object, owner_1_address), 0);

      // owner_2 tries to transfer the token to themself, but fails because they are not the admin
      transfer_example::transfer(owner_2, token_object, owner_2_address);
   }


}
