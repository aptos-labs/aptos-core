#[lint::skip(while_true)]
module 0x815::test {
    public fun f1() {
        'outer: loop {
            // unlabeled loop, but counts in nesting in AST
            loop {
                'inner: loop if (true) loop {
                    if (false) continue 'outer else break 'inner;
                    break
                } else continue 'outer
            };
            break
        }
    }
}
