module middle::middle {
    use leaf::leaf;
    public fun compute(x: u64, y: u64): u64 { leaf::add(x, y) }
}
