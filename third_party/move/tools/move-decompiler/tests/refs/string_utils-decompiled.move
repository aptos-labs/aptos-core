module 0x1::string_utils {
    struct Cons<T0, T1> has copy, drop, store {
        car: T0,
        cdr: T1,
    }
    
    struct FakeCons<T0, T1> has copy, drop, store {
        car: T0,
        cdr: T1,
    }
    
    struct NIL has copy, drop, store {
        dummy_field: bool,
    }
    
    fun cons<T0, T1>(arg0: T0, arg1: T1) : Cons<T0, T1> {
        Cons<T0, T1>{
            car : arg0, 
            cdr : arg1,
        }
    }
    
    public fun debug_string<T0>(arg0: &T0) : 0x1::string::String {
        native_format<T0>(arg0, true, false, false, false)
    }
    
    public fun format1<T0: drop>(arg0: &vector<u8>, arg1: T0) : 0x1::string::String {
        let v0 = cons<T0, NIL>(arg1, nil());
        native_format_list<Cons<T0, NIL>>(arg0, &v0)
    }
    
    public fun format2<T0: drop, T1: drop>(arg0: &vector<u8>, arg1: T0, arg2: T1) : 0x1::string::String {
        let v0 = cons<T0, Cons<T1, NIL>>(arg1, cons<T1, NIL>(arg2, nil()));
        native_format_list<Cons<T0, Cons<T1, NIL>>>(arg0, &v0)
    }
    
    public fun format3<T0: drop, T1: drop, T2: drop>(arg0: &vector<u8>, arg1: T0, arg2: T1, arg3: T2) : 0x1::string::String {
        let v0 = cons<T0, Cons<T1, Cons<T2, NIL>>>(arg1, cons<T1, Cons<T2, NIL>>(arg2, cons<T2, NIL>(arg3, nil())));
        native_format_list<Cons<T0, Cons<T1, Cons<T2, NIL>>>>(arg0, &v0)
    }
    
    public fun format4<T0: drop, T1: drop, T2: drop, T3: drop>(arg0: &vector<u8>, arg1: T0, arg2: T1, arg3: T2, arg4: T3) : 0x1::string::String {
        let v0 = cons<T1, Cons<T2, Cons<T3, NIL>>>(arg2, cons<T2, Cons<T3, NIL>>(arg3, cons<T3, NIL>(arg4, nil())));
        let v1 = cons<T0, Cons<T1, Cons<T2, Cons<T3, NIL>>>>(arg1, v0);
        native_format_list<Cons<T0, Cons<T1, Cons<T2, Cons<T3, NIL>>>>>(arg0, &v1)
    }
    
    native fun native_format<T0>(arg0: &T0, arg1: bool, arg2: bool, arg3: bool, arg4: bool) : 0x1::string::String;
    native fun native_format_list<T0>(arg0: &vector<u8>, arg1: &T0) : 0x1::string::String;
    fun nil() : NIL {
        NIL{dummy_field: false}
    }
    
    public fun to_string<T0>(arg0: &T0) : 0x1::string::String {
        native_format<T0>(arg0, false, false, true, false)
    }
    
    public fun to_string_with_canonical_addresses<T0>(arg0: &T0) : 0x1::string::String {
        native_format<T0>(arg0, false, true, true, false)
    }
    
    public fun to_string_with_integer_types<T0>(arg0: &T0) : 0x1::string::String {
        native_format<T0>(arg0, false, true, true, false)
    }
    
    // decompiled from Move bytecode v6
}
