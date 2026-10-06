// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Picks versions to capture so a corpus covers many entry functions rather than whatever a
//! contiguous range happens to contain: the entry functions called in a version window, and for
//! each the most recent transactions calling it. The indexer answers both questions.

use anyhow::{bail, Context, Result};
use aptos_types::transaction::Version;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};

/// Rows per indexer query, the most the public indexer returns.
const PAGE: usize = 100;

/// Versions per distinct-function query; wider windows time out on the public indexer.
const WINDOW_SLICE: Version = 20_000;

/// What target selection needs from the indexer.
pub trait Indexer {
    /// Up to `limit` distinct entry functions called in `[from, to]`, sorted, after `after`.
    fn entry_functions(
        &self,
        from: Version,
        to: Version,
        after: &str,
        limit: usize,
    ) -> Result<Vec<String>>;

    /// The `limit` most recent versions in `[from, to]` that call `function`.
    fn recent_calls(
        &self,
        function: &str,
        from: Version,
        to: Version,
        limit: usize,
    ) -> Result<Vec<Version>>;
}

/// Versions calling every entry function used in `[from, to]`, up to `per_function` each, in
/// increasing order. Script and other non-entry-function transactions have no function to target.
pub fn select(
    indexer: &dyn Indexer,
    from: Version,
    to: Version,
    per_function: usize,
) -> Result<Vec<Version>> {
    // The indexer times out listing distinct functions over a wide window, so list per slice.
    let mut functions = BTreeSet::new();
    let mut slice_from = from;
    while slice_from <= to {
        let slice_to = to.min(slice_from.saturating_add(WINDOW_SLICE - 1));
        let mut after = String::new();
        let mut cursors = HashSet::new();
        loop {
            let page = indexer.entry_functions(slice_from, slice_to, &after, PAGE)?;
            let Some(last) = page.last() else {
                break;
            };
            // A page that ends where an earlier one did would lead round again for ever. (Only
            // repetition is checked: the indexer orders by its own collation.)
            if !cursors.insert(last.clone()) {
                bail!("the indexer returned no function after {:?}", after);
            }
            after = last.clone();
            let full = page.len() == PAGE;
            functions.extend(page.into_iter().filter(|f| !f.is_empty()));
            if !full {
                break;
            }
        }
        match slice_to.checked_add(1) {
            Some(next) => slice_from = next,
            None => break,
        }
    }
    let mut versions = BTreeSet::new();
    for function in &functions {
        // Within the window only: older calls could predate the framework MonoMove targets. The
        // indexer is held to what was asked: the window, and at most `per_function`.
        let calls = indexer.recent_calls(function, from, to, per_function)?;
        versions.extend(
            calls
                .into_iter()
                .filter(|version| (from..=to).contains(version))
                .take(per_function),
        );
    }
    Ok(versions.into_iter().collect())
}

/// The indexer's GraphQL API.
pub struct GraphQlIndexer {
    url: String,
    api_key: Option<String>,
    client: reqwest::blocking::Client,
}

/// Attempts per query when the indexer rate-limits or times out; the wait doubles each time.
const RATE_LIMIT_ATTEMPTS: u32 = 8;

impl GraphQlIndexer {
    pub fn new(url: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            url: url.into(),
            api_key,
            client: reqwest::blocking::Client::new(),
        }
    }

    fn query<T: for<'de> Deserialize<'de>>(&self, query: &str, variables: Value) -> Result<T> {
        let body = json!({ "query": query, "variables": variables });
        let mut attempt = 0;
        let response = loop {
            let mut request = self.client.post(&self.url).json(&body);
            if let Some(key) = &self.api_key {
                request = request.bearer_auth(key);
            }
            let response = request
                .send()
                .with_context(|| format!("indexer request to {} failed", self.url))?;
            attempt += 1;
            let status = response.status();
            let transient = status == reqwest::StatusCode::TOO_MANY_REQUESTS
                || status == reqwest::StatusCode::REQUEST_TIMEOUT
                || status.is_server_error();
            if !transient || attempt == RATE_LIMIT_ATTEMPTS {
                break response;
            }
            let wait = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok()?.parse().ok())
                .unwrap_or(1u64 << attempt);
            std::thread::sleep(std::time::Duration::from_secs(wait.min(120)));
        };
        let response: Value = response
            .error_for_status()
            .context("indexer returned an error status")?
            .json()
            .context("indexer returned malformed JSON")?;
        if let Some(errors) = response.get("errors") {
            bail!("indexer query failed: {}", errors);
        }
        serde_json::from_value(response["data"]["user_transactions"].clone())
            .context("unexpected indexer response shape")
    }
}

#[derive(Deserialize)]
struct FunctionRow {
    entry_function_id_str: String,
}

#[derive(Deserialize)]
struct VersionRow {
    version: Version,
}

impl Indexer for GraphQlIndexer {
    fn entry_functions(
        &self,
        from: Version,
        to: Version,
        after: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let rows: Vec<FunctionRow> = self.query(
            "query($from: bigint, $to: bigint, $after: String, $limit: Int) {
               user_transactions(
                 where: { version: { _gte: $from, _lte: $to },
                          entry_function_id_str: { _gt: $after } },
                 distinct_on: entry_function_id_str,
                 order_by: { entry_function_id_str: asc },
                 limit: $limit) { entry_function_id_str } }",
            json!({ "from": from, "to": to, "after": after, "limit": limit }),
        )?;
        Ok(rows.into_iter().map(|r| r.entry_function_id_str).collect())
    }

    fn recent_calls(
        &self,
        function: &str,
        from: Version,
        to: Version,
        limit: usize,
    ) -> Result<Vec<Version>> {
        let rows: Vec<VersionRow> = self.query(
            "query($function: String, $from: bigint, $to: bigint, $limit: Int) {
               user_transactions(
                 where: { entry_function_id_str: { _eq: $function },
                          version: { _gte: $from, _lte: $to } },
                 order_by: { version: desc },
                 limit: $limit) { version } }",
            json!({ "function": function, "from": from, "to": to, "limit": limit }),
        )?;
        Ok(rows.into_iter().map(|r| r.version).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Functions `f000`..`f{n}`; function `i` was called at versions `i * 10 ..= i * 10 + 9`.
    struct FakeIndexer {
        functions: Vec<String>,
    }

    impl Indexer for FakeIndexer {
        fn entry_functions(
            &self,
            _: Version,
            _: Version,
            after: &str,
            limit: usize,
        ) -> Result<Vec<String>> {
            Ok(self
                .functions
                .iter()
                .filter(|f| f.as_str() > after)
                .take(limit)
                .cloned()
                .collect())
        }

        fn recent_calls(
            &self,
            function: &str,
            _: Version,
            _: Version,
            limit: usize,
        ) -> Result<Vec<Version>> {
            let i: u64 = function.trim_start_matches('f').parse().expect("index");
            Ok((i * 10..i * 10 + 10).rev().take(limit).collect())
        }
    }

    #[test]
    fn pages_through_every_function() {
        // More functions than one page, plus the empty id scripts have.
        let mut functions: Vec<String> = (0..250).map(|i| format!("f{:03}", i)).collect();
        functions.insert(0, String::new());
        let indexer = FakeIndexer { functions };
        // Three slices, each listing every function: the duplicates merge.
        let versions = select(&indexer, 0, 50_000, 2).expect("select");
        assert_eq!(versions.len(), 500);
        // The two most recent calls of the first and last function.
        assert_eq!(&versions[..2], &[8, 9]);
        assert_eq!(&versions[498..], &[2498, 2499]);
    }

    /// An indexer that ignores `after`: every page is the first.
    struct StuckIndexer;

    impl Indexer for StuckIndexer {
        fn entry_functions(
            &self,
            _: Version,
            _: Version,
            _: &str,
            limit: usize,
        ) -> Result<Vec<String>> {
            Ok((0..limit).map(|i| format!("f{:03}", i)).collect())
        }

        fn recent_calls(&self, _: &str, _: Version, _: Version, _: usize) -> Result<Vec<Version>> {
            Ok(vec![])
        }
    }

    /// An indexer that answers with calls outside the window, and too many.
    struct LooseIndexer;

    impl Indexer for LooseIndexer {
        fn entry_functions(
            &self,
            _: Version,
            _: Version,
            after: &str,
            _: usize,
        ) -> Result<Vec<String>> {
            Ok(if after.is_empty() {
                vec!["f".to_string()]
            } else {
                vec![]
            })
        }

        fn recent_calls(&self, _: &str, _: Version, _: Version, _: usize) -> Result<Vec<Version>> {
            Ok(vec![999, 150, 140, 130, 120, 50])
        }
    }

    #[test]
    fn calls_are_held_to_the_window_and_the_limit() {
        let versions = select(&LooseIndexer, 100, 200, 2).expect("select");
        assert_eq!(versions, vec![140, 150]);
    }

    #[test]
    fn a_page_that_does_not_advance_is_an_error() {
        let err = select(&StuckIndexer, 0, 10, 1).expect_err("stuck");
        assert!(err.to_string().contains("no function after"), "{err}");
    }
}
