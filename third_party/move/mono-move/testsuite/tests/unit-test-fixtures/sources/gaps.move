/// A test MonoMove cannot run: it has no `debug` natives.
module fixtures::gaps {
    #[test]
    fun prints() {
        aptos_std::debug::print(&1u64);
    }
}
