spec aptos_std::event {
    // ****************** SPECIFICATIONS *******************
    spec module {} // switch documentation context to module

    spec module {
        /// Functions of the event module are mocked out using the intrinsic
        /// pragma. They are implemented in the prover's prelude.
        pragma intrinsic = true;

        /// Determines equality between the guids of two event handles. Since fields of intrinsic
        /// structs cannot be accessed, this function is provided.
        fun spec_guid_eq<T>(h1: EventHandle<T>, h2: EventHandle<T>): bool {
            // The implementation currently can just use native equality since the mocked prover
            // representation does not have the `counter` field.
            h1 == h2
        }
    }
}
