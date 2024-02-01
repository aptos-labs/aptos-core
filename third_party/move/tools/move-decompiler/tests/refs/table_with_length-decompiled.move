module 0x1::table_with_length {
    struct TableWithLength<phantom T0: copy + drop, phantom T1> has store {
        inner: 0x1::table::Table<T0, T1>,
        length: u64,
    }
    
    public fun contains<T0: copy + drop, T1>(arg0: &TableWithLength<T0, T1>, arg1: T0) : bool {
        0x1::table::contains<T0, T1>(&arg0.inner, arg1)
    }
    
    public fun add<T0: copy + drop, T1>(arg0: &mut TableWithLength<T0, T1>, arg1: T0, arg2: T1) {
        0x1::table::add<T0, T1>(&mut arg0.inner, arg1, arg2);
        arg0.length = arg0.length + 1;
    }
    
    public fun borrow<T0: copy + drop, T1>(arg0: &TableWithLength<T0, T1>, arg1: T0) : &T1 {
        0x1::table::borrow<T0, T1>(&arg0.inner, arg1)
    }
    
    public fun borrow_mut<T0: copy + drop, T1>(arg0: &mut TableWithLength<T0, T1>, arg1: T0) : &mut T1 {
        0x1::table::borrow_mut<T0, T1>(&mut arg0.inner, arg1)
    }
    
    public fun new<T0: copy + drop, T1: store>() : TableWithLength<T0, T1> {
        TableWithLength<T0, T1>{
            inner  : 0x1::table::new<T0, T1>(), 
            length : 0,
        }
    }
    
    public fun remove<T0: copy + drop, T1>(arg0: &mut TableWithLength<T0, T1>, arg1: T0) : T1 {
        arg0.length = arg0.length - 1;
        0x1::table::remove<T0, T1>(&mut arg0.inner, arg1)
    }
    
    public fun destroy_empty<T0: copy + drop, T1>(arg0: TableWithLength<T0, T1>) {
        assert!(arg0.length == 0, 0x1::error::invalid_state(102));
        let TableWithLength {
            inner  : v0,
            length : _,
        } = arg0;
        0x1::table::destroy<T0, T1>(v0);
    }
    
    public fun empty<T0: copy + drop, T1>(arg0: &TableWithLength<T0, T1>) : bool {
        arg0.length == 0
    }
    
    public fun length<T0: copy + drop, T1>(arg0: &TableWithLength<T0, T1>) : u64 {
        arg0.length
    }
    
    public fun borrow_mut_with_default<T0: copy + drop, T1: drop>(arg0: &mut TableWithLength<T0, T1>, arg1: T0, arg2: T1) : &mut T1 {
        if (0x1::table::contains<T0, T1>(&arg0.inner, arg1)) {
            0x1::table::borrow_mut<T0, T1>(&mut arg0.inner, arg1)
        } else {
            0x1::table::add<T0, T1>(&mut arg0.inner, arg1, arg2);
            arg0.length = arg0.length + 1;
            0x1::table::borrow_mut<T0, T1>(&mut arg0.inner, arg1)
        }
    }
    
    public fun upsert<T0: copy + drop, T1: drop>(arg0: &mut TableWithLength<T0, T1>, arg1: T0, arg2: T1) {
        if (!0x1::table::contains<T0, T1>(&arg0.inner, arg1)) {
            add<T0, T1>(arg0, arg1, arg2);
        } else {
            *0x1::table::borrow_mut<T0, T1>(&mut arg0.inner, arg1) = arg2;
        };
    }
    
    // decompiled from Move bytecode v6
}
