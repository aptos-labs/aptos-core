#[inline(never)]
pub fn recurse(again: bool, value: u32) -> u32 {
    if again { recurse(false, value) } else { value }
}
