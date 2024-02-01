module 0x1::guid {
    struct GUID has drop, store {
        id: ID,
    }
    
    struct ID has copy, drop, store {
        creation_num: u64,
        addr: address,
    }
    
    public(friend) fun create(arg0: address, arg1: &mut u64) : GUID {
        let v0 = *arg1;
        *arg1 = v0 + 1;
        let v1 = ID{
            creation_num : v0, 
            addr         : arg0,
        };
        GUID{id: v1}
    }
    
    public fun create_id(arg0: address, arg1: u64) : ID {
        ID{
            creation_num : arg1, 
            addr         : arg0,
        }
    }
    
    public fun creation_num(arg0: &GUID) : u64 {
        arg0.id.creation_num
    }
    
    public fun creator_address(arg0: &GUID) : address {
        arg0.id.addr
    }
    
    public fun eq_id(arg0: &GUID, arg1: &ID) : bool {
        &arg0.id == arg1
    }
    
    public fun id(arg0: &GUID) : ID {
        arg0.id
    }
    
    public fun id_creation_num(arg0: &ID) : u64 {
        arg0.creation_num
    }
    
    public fun id_creator_address(arg0: &ID) : address {
        arg0.addr
    }
    
    // decompiled from Move bytecode v6
}
