pub fn choose(flag: bool, when_true: u32, when_false: u32) -> u32 {
    if flag { when_true } else { when_false }
}
