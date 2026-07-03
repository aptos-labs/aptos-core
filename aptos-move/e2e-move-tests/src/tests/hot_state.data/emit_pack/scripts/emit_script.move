script {
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::event;
    use 0xcafe::emitter;

    /// Emits an event from a script. `verify_no_event_emission_in_compiled_script` rejects this
    /// *after* `load_script` has already run (and cached the verified script), so the transaction
    /// is kept-and-failed before `validate_and_execute_script`'s immediate-dependency recording is
    /// reached. The script's declared dependencies deliberately mix special-address framework
    /// modules (`coin`, `aptos_coin`, `event`) with a non-special user module (`emitter`).
    fun main() {
        let _ = coin::supply<AptosCoin>();
        event::emit(emitter::make());
    }
}
