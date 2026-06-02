// Two zero-arg #[test] rows with no differentiating expected_failure are redundant.
address 0x1 {
module M {
    #[test]
    #[test]
    fun zero_arg_redundant() {}
}
}
