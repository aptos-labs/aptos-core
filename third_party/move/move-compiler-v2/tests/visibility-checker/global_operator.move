address 0x42 {
module M {
    struct R has key {}
    public fun new(): R {
        R {}
    }
}
module M2 {
    use 0x42::M;
    fun test<Token>(account: signer) {
        let r = M::new();
        borrow_global<M::R>(@0x1);
        borrow_global_mut<M::R>(@0x1);
        exists<M::R>(@0x1);
        move_to(&account, r);
    }
}
}


script {
    use 0x42::M;

    fun test<Token>(account: signer) {
        let r = M::new();
        borrow_global<M::R>(@0x1);
        borrow_global_mut<M::R>(@0x1);
        exists<M::R>(@0x1);
        move_to(&account, r);
    }
}
