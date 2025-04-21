/// This example demonstrates how access control can be decoupled in a secure way from
/// the data model using function values. The module `protected` manages permissions
/// to access data, read or write, in a generic fashion and for all resources in the
/// app. This is not possible without function values, instead, the access control
/// needed to be woven manually into the code for each individual resource. The
/// technical reason why this is not possible because references to storage
/// cannot travel upwards the callstack. However, with function values
/// we can construct continuations which take the value via safe reentrancy
/// as it travels down the stack.

//# publish
module 0x66::protected {
    use 0x1::signer::address_of;

    struct Entry<T>(T) has key;

    public fun create<T:store>(s: &signer, data: T) {
        move_to(s, Entry(data))
    }

    public fun read<T: store, R>(s: &signer, work: |&T|R): R acquires Entry {
        // .. verify read permissions ..
        work(&Entry<T>[address_of(s)].0)
    }

    public fun modify<T: store, R>(s: &signer, work: |&mut T|R): R acquires Entry {
        // .. verify write permissions ..
        work(&mut Entry<T>[address_of(s)].0)
    }
}

//# publish
module 0x66::app {
    use 0x66::protected;

    struct Data(u64) has store;

    fun init_module(s: &signer) {
        protected::create(s, Data(0))
    }

    fun view(s: &signer): u64 {
        protected::read(s, |data: &Data| data.0)
    }

    fun increment(s: &signer): u64 {
        protected::modify(s, |data: &mut Data| { let cur = data.0; data.0 += 1; cur})
    }
}

//# run 0x66::app::init_module --signers 0x66

//# run 0x66::app::increment --signers 0x66

//# run 0x66::app::increment --signers 0x66

//# run 0x66::app::view --signers 0x66
