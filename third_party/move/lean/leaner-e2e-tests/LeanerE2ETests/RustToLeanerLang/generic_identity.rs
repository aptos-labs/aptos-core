pub fn identity<T>(value: T) -> T {
    value
}

pub fn choose<T>(value: &T) -> &T {
    value
}

pub fn round_trip<T>(value: T) -> T {
    identity(value)
}
