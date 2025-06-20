module poc::type_of {
   use aptos_std::type_info;
   use aptos_std::table::Table;


   struct TestStruct has copy, drop, store {
      field1: u64,
      field2: u64,
   }

   public entry fun main(_owner:&signer) {
      let _info_struct = type_info::type_of<TestStruct>();
      let _info_u64 = type_info::type_of<u64>();
      let _info_vec = type_info::type_of<vector<u8>>();
      let _info_table = type_info::type_of<Table<u64, bool>>();
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
