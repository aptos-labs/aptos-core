// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state::State;

#[derive(Debug)]
pub struct Response<T> {
    inner: T,
    state: State,
}

impl<T> Response<T> {
    pub fn new(inner: T, state: State) -> Self {
        Self { inner, state }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn into_parts(self) -> (T, State) {
        (self.inner, self.state)
    }

    pub fn and_then<U, E, F>(self, f: F) -> Result<Response<U>, E>
    where
        F: FnOnce(T) -> Result<U, E>,
    {
        let (inner, state) = self.into_parts();
        match f(inner) {
            Ok(new_inner) => Ok(Response::new(new_inner, state)),
            Err(err) => Err(err),
        }
    }

    pub fn map<U, F>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        let (inner, state) = self.into_parts();
        Response::new(f(inner), state)
    }
}
