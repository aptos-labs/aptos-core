pub fn cast_values(unsigned: u32, signed: i16) -> (u8, u16, i32) {
    (unsigned as u8, signed as u16, signed as i32)
}

