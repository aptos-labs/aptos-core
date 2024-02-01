module 0x1::table {
    struct Box<T0> has drop, store, key {
        val: T0,
    }
    
    struct Table<phantom T0: copy + drop, phantom T1> has store {
        handle: address,
    }
    
    public fun contains<T0: copy + drop, T1>(arg0: &Table<T0, T1>, arg1: T0) : bool {
        contains_box<T0, T1, Box<T1>>(arg0, arg1)
    }
    
    public fun borrow<T0: copy + drop, T1>(arg0: &Table<T0, T1>, arg1: T0) : &T1 {
        &borrow_box<T0, T1, Box<T1>>(arg0, arg1).val
    }
    
    public fun borrow_mut<T0: copy + drop, T1>(arg0: &mut Table<T0, T1>, arg1: T0) : &mut T1 {
        &mut borrow_box_mut<T0, T1, Box<T1>>(arg0, arg1).val
    }
    
    public fun add<T0: copy + drop, T1>(arg0: &mut Table<T0, T1>, arg1: T0, arg2: T1) {
        let v0 = Box<T1>{val: arg2};
        add_box<T0, T1, Box<T1>>(arg0, arg1, v0);
    }
    
    native fun add_box<T0: copy + drop, T1, T2>(arg0: &mut Table<T0, T1>, arg1: T0, arg2: Box<T1>);
    native fun borrow_box<T0: copy + drop, T1, T2>(arg0: &Table<T0, T1>, arg1: T0) : &Box<T1>;
    native fun borrow_box_mut<T0: copy + drop, T1, T2>(arg0: &mut Table<T0, T1>, arg1: T0) : &mut Box<T1>;
    public fun borrow_mut_with_default<T0: copy + drop, T1: drop>(arg0: &mut Table<T0, T1>, arg1: T0, arg2: T1) : &mut T1 {
        if (!contains<T0, T1>(arg0, arg1)) {
            add<T0, T1>(arg0, arg1, arg2);
        };
        borrow_mut<T0, T1>(arg0, arg1)
    }
    
    public fun borrow_with_default<T0: copy + drop, T1>(arg0: &Table<T0, T1>, arg1: T0, arg2: &T1) : &T1 {
        if (!contains<T0, T1>(arg0, arg1)) {
            arg2
        } else {
            borrow<T0, T1>(arg0, arg1)
        }
    }
    
    native fun contains_box<T0: copy + drop, T1, T2>(arg0: &Table<T0, T1>, arg1: T0) : bool;
    public(friend) fun destroy<T0: copy + drop, T1>(arg0: Table<T0, T1>) {
        destroy_empty_box<T0, T1, Box<T1>>(&arg0);
        drop_unchecked_box<T0, T1, Box<T1>>(arg0);
    }
    
    native fun destroy_empty_box<T0: copy + drop, T1, T2>(arg0: &Table<T0, T1>);
    native fun drop_unchecked_box<T0: copy + drop, T1, T2>(arg0: Table<T0, T1>);
    public fun new<T0: copy + drop, T1: store>() : Table<T0, T1> {
        Table<T0, T1>{handle: new_table_handle<T0, T1>()}
    }
    
    native fun new_table_handle<T0, T1>() : address;
    public fun remove<T0: copy + drop, T1>(arg0: &mut Table<T0, T1>, arg1: T0) : T1 {
        let Box { val: v0 } = remove_box<T0, T1, Box<T1>>(arg0, arg1);
        v0
    }
    
    native fun remove_box<T0: copy + drop, T1, T2>(arg0: &mut Table<T0, T1>, arg1: T0) : Box<T1>;
    public fun upsert<T0: copy + drop, T1: drop>(arg0: &mut Table<T0, T1>, arg1: T0, arg2: T1) {
        if (!contains<T0, T1>(arg0, arg1)) {
            add<T0, T1>(arg0, arg1, arg2);
        } else {
            *borrow_mut<T0, T1>(arg0, arg1) = arg2;
        };
    }
    
    // decompiled from Move bytecode v6
}
