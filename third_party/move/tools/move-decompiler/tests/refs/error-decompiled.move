module 0x1::error {
    public fun aborted(arg0: u64) : u64 {
        canonical(7, arg0)
    }
    
    public fun already_exists(arg0: u64) : u64 {
        canonical(8, arg0)
    }
    
    public fun canonical(arg0: u64, arg1: u64) : u64 {
        (arg0 << 16) + arg1
    }
    
    public fun internal(arg0: u64) : u64 {
        canonical(11, arg0)
    }
    
    public fun invalid_argument(arg0: u64) : u64 {
        canonical(1, arg0)
    }
    
    public fun invalid_state(arg0: u64) : u64 {
        canonical(3, arg0)
    }
    
    public fun not_found(arg0: u64) : u64 {
        canonical(6, arg0)
    }
    
    public fun not_implemented(arg0: u64) : u64 {
        canonical(12, arg0)
    }
    
    public fun out_of_range(arg0: u64) : u64 {
        canonical(2, arg0)
    }
    
    public fun permission_denied(arg0: u64) : u64 {
        canonical(5, arg0)
    }
    
    public fun resource_exhausted(arg0: u64) : u64 {
        canonical(9, arg0)
    }
    
    public fun unauthenticated(arg0: u64) : u64 {
        canonical(4, arg0)
    }
    
    public fun unavailable(arg0: u64) : u64 {
        canonical(13, arg0)
    }
    
    // decompiled from Move bytecode v6
}
