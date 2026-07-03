script {
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::event;
    use 0xcafe::emitter;

    /// Event emission gets this script rejected (kept-and-failed) only after `load_script` has
    /// cached the verified script. The dependencies deliberately mix special-address framework
    /// modules with a non-special user module (`emitter`).
    fun main() {
        let _ = coin::supply<AptosCoin>();
        event::emit(emitter::make());
    }
}
