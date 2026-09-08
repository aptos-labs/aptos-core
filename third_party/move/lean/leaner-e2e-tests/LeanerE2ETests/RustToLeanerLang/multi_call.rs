#[inline(never)]
pub fn transform(value: u32) -> u32 {
    value ^ 1
}

pub fn apply(value: u32) -> u32 {
    transform(value)
}

