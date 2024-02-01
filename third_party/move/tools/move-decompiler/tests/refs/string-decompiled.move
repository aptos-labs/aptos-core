module 0x1::string {
    struct String has copy, drop, store {
        bytes: vector<u8>,
    }
    
    public fun index_of(arg0: &String, arg1: &String) : u64 {
        internal_index_of(&arg0.bytes, &arg1.bytes)
    }
    
    public fun append(arg0: &mut String, arg1: String) {
        0x1::vector::append<u8>(&mut arg0.bytes, arg1.bytes);
    }
    
    public fun is_empty(arg0: &String) : bool {
        0x1::vector::is_empty<u8>(&arg0.bytes)
    }
    
    public fun length(arg0: &String) : u64 {
        0x1::vector::length<u8>(&arg0.bytes)
    }
    
    public fun append_utf8(arg0: &mut String, arg1: vector<u8>) {
        append(arg0, utf8(arg1));
    }
    
    public fun bytes(arg0: &String) : &vector<u8> {
        &arg0.bytes
    }
    
    public fun insert(arg0: &mut String, arg1: u64, arg2: String) {
        let v0 = &arg0.bytes;
        assert!(arg1 <= 0x1::vector::length<u8>(v0) && internal_is_char_boundary(v0, arg1), 2);
        let v1 = sub_string(arg0, 0, arg1);
        append(&mut v1, arg2);
        append(&mut v1, sub_string(arg0, arg1, length(arg0)));
        *arg0 = v1;
    }
    
    native public fun internal_check_utf8(arg0: &vector<u8>) : bool;
    native fun internal_index_of(arg0: &vector<u8>, arg1: &vector<u8>) : u64;
    native fun internal_is_char_boundary(arg0: &vector<u8>, arg1: u64) : bool;
    native fun internal_sub_string(arg0: &vector<u8>, arg1: u64, arg2: u64) : vector<u8>;
    public fun sub_string(arg0: &String, arg1: u64, arg2: u64) : String {
        let v0 = &arg0.bytes;
        let v1 = arg2 <= 0x1::vector::length<u8>(v0);
        assert!(v1 && arg1 <= arg2 && internal_is_char_boundary(v0, arg1) && internal_is_char_boundary(v0, arg2), 2);
        String{bytes: internal_sub_string(v0, arg1, arg2)}
    }
    
    public fun try_utf8(arg0: vector<u8>) : 0x1::option::Option<String> {
        if (internal_check_utf8(&arg0)) {
            let v1 = String{bytes: arg0};
            0x1::option::some<String>(v1)
        } else {
            0x1::option::none<String>()
        }
    }
    
    public fun utf8(arg0: vector<u8>) : String {
        assert!(internal_check_utf8(&arg0), 1);
        String{bytes: arg0}
    }
    
    // decompiled from Move bytecode v6
}
