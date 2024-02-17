module NamedAddr::Detector {
    public fun loop_with_while_true() {
        let counter = 0;
        while (true) { // Should trigger a warning
            if (counter >= 10) { break };
            counter = counter + 1;
        }
    }

    // public fun loop_with_condition() {
    //     let mut counter = 0;
    //     while (counter < 10) { // Correct usage
    //         counter = counter + 1;
    //     }
    // }
}
