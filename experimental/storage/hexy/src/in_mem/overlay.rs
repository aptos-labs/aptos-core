// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    in_mem::{base::HexyBase, view::HexyView},
    NodePosition,
};
use aptos_crypto::HashValue;
use aptos_experimental_layered_map::MapLayer;
use std::sync::Arc;

#[derive(Clone)]
pub struct HexyOverlay {
    pub(crate) overlay: MapLayer<NodePosition, HashValue>,
    /// Can be derived if a View is constructed, but storing it here might make it easier.
    pub(crate) root_hash: HashValue,
}

impl HexyOverlay {
    pub fn new_empty(base: &Arc<HexyBase>) -> Self {
        let overlay = MapLayer::new_family("hexy");
        let root_hash = base.root_hash();

        Self { overlay, root_hash }
    }

    pub fn view(&self, base: &Arc<HexyBase>, base_overlay: &HexyOverlay) -> HexyView {
        HexyView::new(
            base.clone(),
            self.overlay.view_layers_after(&base_overlay.overlay),
        )
    }

    pub fn root_hash(&self) -> HashValue {
        self.root_hash
    }

    pub fn is_the_same(&self, other: &Self) -> bool {
        self.overlay.is_the_same(&other.overlay)
    }
}
