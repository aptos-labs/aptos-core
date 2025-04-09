module poc::move_range {
   use std::vector;

   public entry fun main(_owner: &signer) {
      let v_src = vector[1u64, 2u64, 3u64, 4u64];
      let v_dst = vector::empty<u64>();
      vector::move_range(&mut v_src, 1, 2, &mut v_dst, 0);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
