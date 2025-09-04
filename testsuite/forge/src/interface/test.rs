// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use rand::SeedableRng;
use std::borrow::Cow;

/// Whether a test is expected to fail or not
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShouldFail {
    No,
    Yes,
    YesWithMessage(&'static str),
}

#[derive(Debug, Clone)]
pub struct TestDetails {
    pub name: String,
    pub reporting_name: String,
}

impl TestDetails {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn reporting_name(&self) -> String {
        self.reporting_name.clone()
    }
}

/// Represents a Test in Forge
///
/// This is meant to be a super trait of the other test interfaces.
pub trait Test: Send + Sync {
    /// Returns the name of the Test
    fn name(&self) -> &'static str;

    /// Indicates if the Test should be ignored
    fn ignored(&self) -> bool {
        false
    }

    /// Indicates if the Test should fail
    fn should_fail(&self) -> ShouldFail {
        ShouldFail::No
    }

    /// Name used specifically for external reporting
    fn reporting_name(&self) -> Cow<'static, str> {
        Cow::Borrowed(self.name())
    }

    fn details(&self) -> TestDetails {
        TestDetails {
            name: self.name().to_string(),
            reporting_name: self.reporting_name().to_string(),
        }
    }
}

impl<T: Test + ?Sized> Test for &T {
    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn ignored(&self) -> bool {
        (**self).ignored()
    }

    fn should_fail(&self) -> ShouldFail {
        (**self).should_fail()
    }
}

#[derive(Debug)]
pub struct CoreContext {
    rng: ::rand::rngs::StdRng,
}

impl CoreContext {
    pub fn new(rng: ::rand::rngs::StdRng) -> Self {
        Self { rng }
    }

    pub fn from_rng<R: ::rand::RngCore + ::rand::CryptoRng>(rng: R) -> Self {
        Self {
            rng: ::rand::rngs::StdRng::from_rng(rng).unwrap(),
        }
    }

    pub fn rng(&mut self) -> &mut ::rand::rngs::StdRng {
        &mut self.rng
    }
}
