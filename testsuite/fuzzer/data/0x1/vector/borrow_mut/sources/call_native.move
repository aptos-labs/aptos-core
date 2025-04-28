module poc::borrow_mut {
   use std::vector;

   public entry fun main(_owner:&signer) {
      let v = vector[1u64, 2u64, 3u64];
      let elem_mut_ref = vector::borrow_mut(&mut v, 1);
      *elem_mut_ref = 100;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
