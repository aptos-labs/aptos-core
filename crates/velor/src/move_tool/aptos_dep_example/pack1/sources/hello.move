module project::test {
    use project::m;

    public entry fun test(_sender: &signer) {
        assert!(m::add(1, 2) == 1 + 2, 1);
    }
}
