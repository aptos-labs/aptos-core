module NamedAddr::Detector {
    // Function with non-redundant boolean expression
    public fun non_redundant_expression(x: bool): bool {
        x && true // Should not trigger a warning
    }

    // Function with redundant boolean expression
    public fun redundant_expression(x: bool): bool {
        true || x
    }

    // Function with redundant boolean expression
    public fun redundant_expression2(x: bool): bool {
        false || x // Should trigger a warning
    }
}
