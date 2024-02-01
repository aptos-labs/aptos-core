module std::Types {
    struct Struct1 {
        val_bool: bool,
        val_u8: u8,
        val_u16: u16,
        val_u32: u32,
        val_u64: u64,
        val_u128: u128,
        val_u256: u256,
        val_address: address,
    }

    struct Struct2 {
        val_vec_bool: vector<bool>,
        val_vec_u8: vector<u8>,
        val_vec_u16: vector<u16>,
        val_vec_u32: vector<u32>,
        val_vec_u64: vector<u64>,
        val_vec_u128: vector<u128>,
        val_vec_u256: vector<u256>,
        val_vec_address: vector<address>,
    }

    struct Struct3 {
        val_struct1: Struct1,
        val_struct2: Struct2,
    }

    struct Struct4 {
        val_vec_struct1: vector<Struct1>,
        val_vec_struct2: vector<Struct2>,
        val_vec_struct3: vector<Struct3>,
    }

    struct Struct5<T> {
        val_vec_T: vector<T>,
    }

    struct Struct6<T: copy + drop + store, U: copy + store + key> has copy, store {
        val_T: T,
        val_U: U,
    }

    struct Struct7 has copy, drop, store {
        val: u8,
    }

    struct Struct8 has copy, store, key {
        val: u8,
    }

    struct Struct9 has copy, store {
        val: Struct6<Struct7, Struct8>,
    }
    

    public fun dummy() {
    }
}