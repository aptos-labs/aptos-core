module poc::push_back {
   use std::vector;

   public entry fun main(_owner: &signer) {
      let v = vector::empty<u64>();
      vector::push_back(&mut v, 42u64);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
