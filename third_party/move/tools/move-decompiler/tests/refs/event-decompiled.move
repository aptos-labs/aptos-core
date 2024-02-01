module 0x1::event {
    struct EventHandle<phantom T0: drop + store> has store {
        counter: u64,
        guid: 0x1::guid::GUID,
    }
    
    public fun guid<T0: drop + store>(arg0: &EventHandle<T0>) : &0x1::guid::GUID {
        &arg0.guid
    }
    
    public fun counter<T0: drop + store>(arg0: &EventHandle<T0>) : u64 {
        arg0.counter
    }
    
    public fun destroy_handle<T0: drop + store>(arg0: EventHandle<T0>) {
        let EventHandle {
            counter : _,
            guid    : _,
        } = arg0;
    }
    
    public fun emit<T0: drop + store>(arg0: T0) {
        write_module_event_to_store<T0>(arg0);
    }
    
    public fun emit_event<T0: drop + store>(arg0: &mut EventHandle<T0>, arg1: T0) {
        write_to_event_store<T0>(0x1::bcs::to_bytes<0x1::guid::GUID>(&arg0.guid), arg0.counter, arg1);
        arg0.counter = arg0.counter + 1;
    }
    
    public(friend) fun new_event_handle<T0: drop + store>(arg0: 0x1::guid::GUID) : EventHandle<T0> {
        EventHandle<T0>{
            counter : 0, 
            guid    : arg0,
        }
    }
    
    native fun write_module_event_to_store<T0: drop + store>(arg0: T0);
    native fun write_to_event_store<T0: drop + store>(arg0: vector<u8>, arg1: u64, arg2: T0);
    // decompiled from Move bytecode v6
}
