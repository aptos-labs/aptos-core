module 0xA550C18::HelloWorld {
    use AptosFramework::Account;

    public fun foo(addr: address): u64 {
        Account::get_balance(addr)
    }
}
