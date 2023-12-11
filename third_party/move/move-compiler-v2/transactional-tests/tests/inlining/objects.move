//# publish

// A module which provides the functions to manage objects.
//
// Those are provided as inline functions which are inlined in the context of concrete object implementation.
module 0x42::objects {

    // ================================================
    // Reference types

    // Need to have public accessors for all of them so they can be accessed from outside this module
    // in inlining context

    struct OwnerRef has drop { // Hot potato -- cannot be stored
        addr: address, // This should be a SignerCapability, in reality
    }
    public fun make_owner_ref(addr: address): OwnerRef { // for this test only
        OwnerRef{addr}
    }
    public fun owner_addr_of(r: &OwnerRef): address {
        r.addr
    }

    struct ReaderRef<phantom T: key> has store, drop {
        addr: address
    }

    public fun make_reader_ref<T: key>(addr: address): ReaderRef<T> {
        ReaderRef<T>{addr}
    }
    public fun reader_addr_of<T: key>(r: &ReaderRef<T>): address {
        r.addr
    }

    struct WriterRef<phantom T: key> has store, drop {
        addr: address,
    }

    public fun make_writer_ref<T: key>(addr: address): WriterRef<T> {
        WriterRef{addr}
    }

    public fun writer_addr_of<T: key>(r: &WriterRef<T>): address {
        r.addr
    }

    // ================================================
    // Basic Operations, inlined in the object definition context

    public inline fun create<T: key>(signer: &signer, _ref: &OwnerRef, val: T) {
        // In reality, we should get the signer via the ref
        move_to<T>(signer, val)
    }

    public inline fun reader_ref<T: key>(ref: &OwnerRef): ReaderRef<T> {
        let addr = owner_addr_of(ref);
        assert!(exists<T>(addr), 22);
        make_reader_ref(addr)
    }

    public inline fun writer_ref<T: key>(ref: &OwnerRef): WriterRef<T> {
        let addr = owner_addr_of(ref);
        assert!(exists<T>(addr), 23);
        make_writer_ref(addr)
    }

    public inline fun reader<T: key>(ref: &ReaderRef<T>): &T {
        borrow_global<T>(reader_addr_of(ref))
    }

    public inline fun writer<T: key>(ref: &WriterRef<T>): &mut T {
        borrow_global_mut<T>(writer_addr_of(ref))
    }
}

//# publish
module 0x42::token {
    use 0x42::objects as obj;

    struct Token has key { val: u64 }

    public fun create(signer: &signer, owner: &obj::OwnerRef, val: u64) {
        obj::create(signer, owner, Token{val})
    }

    public fun reader_ref(r: &obj::OwnerRef): obj::ReaderRef<Token> {
        obj::reader_ref<Token>(r)
    }

    public fun writer_ref(r: &obj::OwnerRef): obj::WriterRef<Token> {
        obj::writer_ref<Token>(r)
    }

    public fun get_value(ref: &obj::ReaderRef<Token>): u64 acquires Token {
        obj::reader(ref).val
    }

    public fun set_value(ref: &obj::WriterRef<Token>, val: u64) acquires Token {
        obj::writer(ref).val = val
    }
}

//# run --signers 0x42
script {
    use 0x42::token;
    fun main(s: signer) {
        let or = 0x42::objects::make_owner_ref(@0x42);
        token::create(&s, &or, 22);
        let rr = token::reader_ref(&or);
        let wr = token::writer_ref(&or);
        assert!(token::get_value(&rr) == 22, 0);
        token::set_value(&wr, 23);
        assert!(token::get_value(&rr) == 23, 1)
    }
}
