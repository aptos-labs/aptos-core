pub fn get(values: [u32; 4], index: usize) -> u32 {
    values[index]
}

pub fn get_third(values: [u32; 4]) -> u32 {
    values[2]
}

pub fn destructure(values: [u32; 4]) -> u32 {
    let [first, _, _, _] = values;
    first
}
