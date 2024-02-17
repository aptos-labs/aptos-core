module NamedAddr::Detector {
    public fun correct_usage() {
        let a = 5;
        let b = 10;
        a = b; // Correct usage
    }

    public fun self_assignment() {
        let a = 5;
        a = a; // Should trigger a warning
    }
}
