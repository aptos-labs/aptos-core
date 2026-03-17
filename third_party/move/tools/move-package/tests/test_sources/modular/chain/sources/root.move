module root::root {
    use middle::middle;
    public fun run(x: u64, y: u64): u64 { middle::compute(x, y) }
}
