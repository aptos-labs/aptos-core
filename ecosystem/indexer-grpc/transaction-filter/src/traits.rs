// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::errors::FilterError;
use serde::Serialize;
use std::fmt::Debug;

/// Simple trait to allow for filtering of items of type T
pub trait Filterable<T>
where
    Self: Debug + Serialize,
{
    /// Whether this filter is correctly configured/initialized
    /// Any call to `validate_state` is responsible for recursively checking the validity of any nested filters *by calling `is_valid`*
    /// The actual public API is via `is_valid` which will call `validate_state` and return an error if it fails, but annotated with the filter type/path
    fn validate_state(&self) -> Result<(), FilterError>;

    /// This is a convenience method to allow for the error to be annotated with the filter type/path at each level
    /// This is the public API for checking the validity of a filter!
    /// Example output looks like:
    /// ```text
    /// FilterError: This is a test error!.
    /// Trace Path:
    /// transaction_filter::traits::test::InnerStruct:   {"a":"test"}
    /// core::option::Option<transaction_filter::traits::test::InnerStruct>:   {"a":"test"}
    /// transaction_filter::traits::test::OuterStruct:   {"inner":{"a":"test"}}
    ///  ```
    ///
    #[inline]
    fn is_valid(&self) -> Result<(), FilterError> {
        // T
        self.validate_state().map_err(|mut e| {
            e.add_trace(
                serde_json::to_string(self).unwrap(),
                std::any::type_name::<Self>().to_string(),
            );
            e
        })
    }

    /// Whether the item is allowed by this filter
    /// This is the core method that should be implemented by any filter
    /// This is the method that should be called by any parent filter to determine if an item is allowed
    /// *If a filter doesn't explicitly prevent an item, then it should be allowed*
    /// This forces the logic of `if !child_filter.matches(item) { return false; }` for any parent filter
    fn matches(&self, item: &T) -> bool;

    #[inline]
    fn matches_vec(&self, items: &[T]) -> bool {
        items.iter().any(|item| self.matches(item))
    }

    #[inline]
    fn matches_opt(&self, item: &Option<T>) -> bool {
        match item {
            Some(item) => self.matches(item),
            None => false,
        }
    }

    #[inline]
    fn matches_opt_vec(&self, items: &Option<&Vec<T>>) -> bool {
        match items {
            Some(items) => self.matches_vec(items),
            None => false,
        }
    }

    #[inline]
    fn filter_vec(&self, items: Vec<T>) -> Vec<T> {
        items
            .into_iter()
            .filter(|item| self.matches(item))
            .collect()
    }
}

/// This allows for `Option<Filterable>` to always return true: i.e if the filter is None, then all items are allowed.
impl<T, F> Filterable<T> for Option<F>
where
    F: Filterable<T>,
{
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        match self {
            Some(filter) => filter.is_valid(),
            None => Ok(()),
        }
    }

    #[inline]
    fn matches(&self, item: &T) -> bool {
        match self {
            Some(filter) => filter.matches(item),
            None => true,
        }
    }

    #[inline]
    fn matches_opt(&self, item: &Option<T>) -> bool {
        match self {
            Some(filter) => filter.matches_opt(item),
            None => true,
        }
    }
}

impl Filterable<String> for Option<String> {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        Ok(())
    }

    #[inline]
    fn matches(&self, item: &String) -> bool {
        match self {
            Some(filter) => filter == item,
            None => true,
        }
    }
}

impl Filterable<i32> for Option<i32> {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        Ok(())
    }

    #[inline]
    fn matches(&self, item: &i32) -> bool {
        match self {
            Some(filter) => filter == item,
            None => true,
        }
    }
}

impl Filterable<bool> for Option<bool> {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        Ok(())
    }

    #[inline]
    fn matches(&self, item: &bool) -> bool {
        match self {
            Some(filter) => filter == item,
            None => true,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::anyhow;

    #[derive(Debug, Serialize, PartialEq)]
    pub struct InnerStruct {
        pub a: Option<String>,
    }

    impl Filterable<InnerStruct> for InnerStruct {
        fn validate_state(&self) -> Result<(), FilterError> {
            Err(anyhow!("This is a test error!").into())
        }

        fn matches(&self, _item: &InnerStruct) -> bool {
            true
        }
    }

    #[derive(Debug, PartialEq, Serialize)]
    pub struct OuterStruct {
        pub inner: Option<InnerStruct>,
    }

    impl Filterable<InnerStruct> for OuterStruct {
        fn validate_state(&self) -> Result<(), FilterError> {
            self.inner.is_valid()?;
            Ok(())
        }

        fn matches(&self, item: &InnerStruct) -> bool {
            self.inner.matches(item)
        }
    }

    #[test]
    fn test_error_prop() {
        let inner = InnerStruct {
            a: Some("test".to_string()),
        };
        let outer = OuterStruct { inner: Some(inner) };

        let res = outer.is_valid();
        assert!(res.is_err());
        let error = res.unwrap_err();
        assert_eq!(error.to_string(), "Filter Error: This is a test error!\nTrace Path:\nvelor_transaction_filter::traits::test::InnerStruct:   {\"a\":\"test\"}\ncore::option::Option<velor_transaction_filter::traits::test::InnerStruct>:   {\"a\":\"test\"}\nvelor_transaction_filter::traits::test::OuterStruct:   {\"inner\":{\"a\":\"test\"}}");
    }
}
