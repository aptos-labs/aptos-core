#[inline(never)]
pub fn read(value: &u32) -> u32 {
    *value
}

pub fn borrow_then_read(value: u32) -> u32 {
    read(&value)
}

