module poc::is_identifier {
    use std::string;
    use aptos_framework::function_info;

    public entry fun main(_owner:&signer) {
        let module_name = string::utf8(b"valid_module");
        let function_name = string::utf8(b"valid_function");
        let _info = function_info::new_function_info_from_address(@0xcaffe, module_name, function_name);
    }

   #[test(owner=@0x123)]
   fun a(owner:&signer){
      main(owner);
    }
}
