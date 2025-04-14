module poc::type_name {
   use aptos_std::type_info;
   use std::string::{String};
   use aptos_std::table::Table;

   struct MyStruct has copy, drop, store {
       x: u64,
       y: bool,
   }

   public entry fun main(_owner: &signer) {
      let name_u64: String = type_info::type_name<u64>();
      let _ = name_u64;

      let name_bool: String = type_info::type_name<bool>();
      let _ = name_bool;

      let name_addr: String = type_info::type_name<address>();
      let _ = name_addr;

      let name_vec_u8: String = type_info::type_name<vector<u8>>();
      let _ = name_vec_u8;

      let name_vec_addr: String = type_info::type_name<vector<address>>();
      let _ = name_vec_addr;

      let name_struct: String = type_info::type_name<MyStruct>();
      let _ = name_struct;

      let name_generic_struct: String = type_info::type_name<Table<u64, bool>>();
      let _ = name_generic_struct;

      let name_nested_vec: String = type_info::type_name<vector<vector<MyStruct>>>();
      let _ = name_nested_vec;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
