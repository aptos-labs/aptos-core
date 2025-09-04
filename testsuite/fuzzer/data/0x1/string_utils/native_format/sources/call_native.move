module poc::native_format {
   use velor_std::string_utils;
   use std::vector;
   use std::signer;

   struct TestStruct has store, drop, copy {
      val_u64: u64,
      val_u8: u8,
      val_u16: u16,
      val_u32: u32,
      val_u128: u128,
   }

   struct NestedStruct has store, drop, copy {
       depth1: vector<TestStruct>,
       val: u64
   }

   public entry fun main(owner: &signer) {
      let val_u8 = 8u8;
      let val_u16 = 16u16;
      let val_u32 = 32u32;
      let val_u64 = 64u64;
      let val_u128 = 128u128;

      let addr = signer::address_of(owner);
      let owner_ref = owner;

      let simple_struct = TestStruct {
          val_u64: val_u64,
          val_u8: val_u8,
          val_u16: val_u16,
          val_u32: val_u32,
          val_u128: val_u128,
      };

      let _s_u8 = string_utils::to_string(&val_u8);
      let _s_u16 = string_utils::to_string(&val_u16);
      let _s_u32 = string_utils::to_string(&val_u32);
      let _s_u64 = string_utils::to_string(&val_u64);
      let _s_u128 = string_utils::to_string(&val_u128);
      let _s_addr = string_utils::to_string(&addr);
      let _s_signer = string_utils::to_string(owner_ref);
      let _s_signer_canon = string_utils::to_string_with_canonical_addresses(owner_ref);
      let _s_addr_canon = string_utils::to_string_with_canonical_addresses(&addr);
      let _s_debug_u128 = string_utils::debug_string(&val_u128);

      let _s_struct = string_utils::to_string(&simple_struct);
      let _s_struct_canon = string_utils::to_string_with_canonical_addresses(&simple_struct);
      let _s_struct_int = string_utils::to_string_with_integer_types(&simple_struct);
      let _s_struct_debug = string_utils::debug_string(&simple_struct);

      let nested3 = NestedStruct { depth1: vector::singleton(copy simple_struct), val: 3 };
      let _s_nested = string_utils::debug_string(&nested3);

      let long_vector = vector::empty<u64>();
      let i = 0u64;
      while (i < 100) {
          vector::push_back(&mut long_vector, i);
          i = i + 1;
      };
      let _s_long_vec = string_utils::debug_string(&long_vector);
      let _s_long_vec_simple = string_utils::to_string(&long_vector);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
