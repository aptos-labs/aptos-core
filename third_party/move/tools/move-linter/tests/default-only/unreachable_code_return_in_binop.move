#[lint::skip(simpler_numeric_expression, nonminimal_bool, known_to_abort, unnecessary_numerical_extreme_comparison)]
module 0x42::M {
    public fun t() {
        return >> 0;
        return << 0;
        return || false;
        return && false;
        return + 0;
        return % 0;
        return / 1;
        return < 0;
        return > 0;
        return <= 0;
        return == 0;
        return >= 0;
        return != 0;
        return | 0;
        return ^ 0;
    }
}
