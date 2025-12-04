// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{account_config::Table, event::EventHandle};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingClaimsResource {
    pending_claims: Table,
    offer_events: EventHandle,
    cancel_offer_events: EventHandle,
    claim_events: EventHandle,
}

impl PendingClaimsResource {
    pub fn new(
        pending_claims: Table,
        offer_events: EventHandle,
        cancel_offer_events: EventHandle,
        claim_events: EventHandle,
    ) -> Self {
        Self {
            pending_claims,
            offer_events,
            cancel_offer_events,
            claim_events,
        }
    }

    pub fn pending_claims(&self) -> &Table {
        &self.pending_claims
    }

    pub fn offer_events(&self) -> &EventHandle {
        &self.offer_events
    }

    pub fn cancel_offer_events(&self) -> &EventHandle {
        &self.cancel_offer_events
    }

    pub fn claim_events(&self) -> &EventHandle {
        &self.claim_events
    }
}

impl MoveStructType for PendingClaimsResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("PendingClaims");
}

impl MoveResource for PendingClaimsResource {}
