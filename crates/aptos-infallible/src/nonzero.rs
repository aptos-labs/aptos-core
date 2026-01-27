// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// A wrapper around `std::num::NonZeroUsize` to no longer worry about `unwrap()`
#[macro_export]
macro_rules! NonZeroUsize {
    ($num:expr) => {
        NonZeroUsize!($num, "Must be non-zero")
    };
    ($num:expr, $message:literal) => {
        std::num::NonZeroUsize::new($num).expect($message)
    };
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_nonzero() {
        assert_eq!(1, NonZeroUsize!(1).get());
        assert_eq!(usize::MAX, NonZeroUsize!(usize::MAX).get());
    }

    #[test]
    #[should_panic(expected = "Must be non-zero")]
    fn test_zero() {
        NonZeroUsize!(0);
    }

    #[test]
    #[should_panic(expected = "Custom message")]
    fn test_zero_custom_message() {
        NonZeroUsize!(0, "Custom message");
    }
}
