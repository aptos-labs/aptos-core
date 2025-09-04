module poc::chain_id_internal {
   use velor_std::type_info;

   public entry fun main(_owner: &signer) {
      let id: u8 = type_info::chain_id();
      let _ = id;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
