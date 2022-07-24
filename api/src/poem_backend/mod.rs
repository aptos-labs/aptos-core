// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::Tags;

mod accept_type;
mod accounts;
mod basic;
mod bcs_payload;
mod events;
mod index;
mod log;
mod page;
mod post;
mod response;
mod runtime;
mod state;
mod transactions;

#[derive(Tags)]
pub enum ApiTags {
    /// Access to account resources and modules
    Accounts,

    /// Access to events
    Events,

    /// General information
    General,

    /// Access to tables
    Tables,

    /// Access to transactions
    Transactions,
}

pub use accept_type::AcceptType;
pub use accounts::AccountsApi;
pub use basic::BasicApi;
pub use events::EventsApi;
pub use index::IndexApi;
pub use log::middleware_log;
pub use post::AptosPost;
pub use response::*;
pub use runtime::attach_poem_to_runtime;
pub use transactions::TransactionsApi;

// TODO: Move these impls throughout each of the files in the parent directory.
// The only reason I do it here right now is the existing handler functions return
// opaque reply objects and therefore I can't re-use them, so I'd have to pollute
// those files with these impls below.
