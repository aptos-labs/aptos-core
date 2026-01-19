module 0x816::ops {
    use 0x815::types::Color;
    use 0x815::types::make_color;
    public entry fun check(r: u8, result: u64) {
        let c = make_color(r);
        assert!(match (c) { Color::Red { r } => (r as u64), Color::Green { g } => (g as u64) } == result, 1);
    }
}
