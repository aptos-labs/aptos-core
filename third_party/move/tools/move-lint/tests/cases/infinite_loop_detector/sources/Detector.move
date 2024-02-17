module NamedAddr::Detector {
     public fun infinite_loop() {
        loop { // Should trigger a warning
            // No break or return statement
      
        }
    }

    public fun finite_loop() {
        let counter = 0;
        while (true) { // Correct usage
            if (counter >= 10) { break };
            counter = counter + 1;
        }
    }
}
