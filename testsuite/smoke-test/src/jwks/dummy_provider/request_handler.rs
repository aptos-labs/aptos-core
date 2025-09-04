// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_infallible::Mutex;
use hyper::{Body, Request};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, str::FromStr};

/// A handler that handles JWK requests from a validator,
/// assuming the validator account address is written as the COOKIE.
pub trait RequestHandler: Send + Sync {
    fn handle(&self, request: Request<Body>) -> Vec<u8>;
}

pub struct StaticContentServer {
    content: Vec<u8>,
}

impl StaticContentServer {
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }

    pub fn new_str(content: &str) -> Self {
        Self::new(content.as_bytes().to_vec())
    }
}

impl RequestHandler for StaticContentServer {
    fn handle(&self, _origin: Request<Body>) -> Vec<u8> {
        self.content.clone()
    }
}

fn origin_from_cookie(request: &Request<Body>) -> AccountAddress {
    let cookie = request
        .headers()
        .get(hyper::header::COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    AccountAddress::from_str(cookie).unwrap()
}

/// The first `k` requesters will get content A forever, the rest will get content B forever.
pub struct EquivocatingServer {
    content_a: Vec<u8>,
    content_b: Vec<u8>,
    k: usize,
    requesters_observed: Mutex<HashSet<AccountAddress>>,
}

impl EquivocatingServer {
    pub fn new(content_a: Vec<u8>, content_b: Vec<u8>, k: usize) -> Self {
        Self {
            content_a,
            content_b,
            k,
            requesters_observed: Mutex::new(HashSet::new()),
        }
    }
}

impl RequestHandler for EquivocatingServer {
    fn handle(&self, request: Request<Body>) -> Vec<u8> {
        let mut requesters_observed = self.requesters_observed.lock();
        let origin = origin_from_cookie(&request);
        if requesters_observed.len() < self.k {
            requesters_observed.insert(origin);
        }

        if requesters_observed.contains(&origin) {
            self.content_a.clone()
        } else {
            self.content_b.clone()
        }
    }
}

/// This server first replies with `initial_thoughts`.
/// After enough audience receives it for at least once, it switches its reply to `second_thoughts`.
///
/// This behavior simulates the situation where a provider performs a 2nd key rotation right after the 1st.
pub struct MindChangingServer {
    initial_thoughts: Vec<u8>,
    second_thoughts: Vec<u8>,
    change_mind_threshold: usize,
    requesters_observed: Mutex<HashSet<AccountAddress>>,
}

impl MindChangingServer {
    pub fn new(
        initial_thoughts: Vec<u8>,
        second_thoughts: Vec<u8>,
        change_mind_threshold: usize,
    ) -> Self {
        Self {
            initial_thoughts,
            second_thoughts,
            change_mind_threshold,
            requesters_observed: Mutex::new(HashSet::new()),
        }
    }
}

impl RequestHandler for MindChangingServer {
    fn handle(&self, request: Request<Body>) -> Vec<u8> {
        let mut requesters_observed = self.requesters_observed.lock();
        let origin = origin_from_cookie(&request);
        if requesters_observed.contains(&origin)
            || requesters_observed.len() >= self.change_mind_threshold
        {
            self.second_thoughts.clone()
        } else {
            requesters_observed.insert(origin);
            self.initial_thoughts.clone()
        }
    }
}
