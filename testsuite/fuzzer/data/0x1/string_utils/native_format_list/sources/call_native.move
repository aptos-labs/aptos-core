module poc::native_format_list {
   use aptos_std::string_utils;
   use std::string::{String};

   public entry fun main(_owner: &signer) {
      let val = 42u64;

      let fmt1 = b"Value: {}";
      let _sf1: String = string_utils::format1(&fmt1, val);

      let fmt2 = b"Values: {}, {}";
      let _sf2: String = string_utils::format2(&fmt2, val, true);

      let fmt3 = b"Values: {}, {}, {}";
      let _sf3: String = string_utils::format3(&fmt3, val, true, @0x1);

      let fmt4 = b"Values: {}, {}, {}, {}";
      let _sf4: String = string_utils::format4(&fmt4, val, true, @0x1, vector[1u8]);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
