// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reader over the committed position, order and collateral tips together.
//!
//! Pre-commit publishes the three tips under all three locks, so holding
//! them for the scope pins every store to the same chunk.

#![forbid(unsafe_code)]

use crate::db::{CollateralBundle, OrderBundle, PositionBundle};
use aptos_storage_interface::state_store::{
    collateral::CollateralView, orders::OrderView, positions::NativeStateView,
};
use std::sync::Arc;

/// The stores attached to this DB, each read at the same chunk.
pub struct TradingNativeView<'a> {
    pub positions: Option<&'a NativeStateView<'a>>,
    pub orders: Option<&'a OrderView<'a>>,
    pub collateral: Option<&'a CollateralView<'a>>,
}

pub struct TradingNativeReader {
    position: Option<Arc<PositionBundle>>,
    order: Option<Arc<OrderBundle>>,
    collateral: Option<Arc<CollateralBundle>>,
}

impl TradingNativeReader {
    pub(crate) fn new(
        position: Option<Arc<PositionBundle>>,
        order: Option<Arc<OrderBundle>>,
        collateral: Option<Arc<CollateralBundle>>,
    ) -> Self {
        Self {
            position,
            order,
            collateral,
        }
    }

    /// Takes the tips in publish order, then each base. A fold doesn't
    /// change what a view reads, so the bases need not be taken together.
    pub fn with_view<R>(&self, f: impl FnOnce(&TradingNativeView<'_>) -> R) -> R {
        let position_tip = self.position.as_ref().map(|bundle| bundle.positions.lock());
        let order_tip = self.order.as_ref().map(|bundle| bundle.orders.lock());
        let collateral_tip = self
            .collateral
            .as_ref()
            .map(|bundle| bundle.collateral.lock());

        let with_collateral = |positions: Option<&NativeStateView<'_>>,
                               orders: Option<&OrderView<'_>>| {
            match (self.collateral.as_ref(), collateral_tip.as_ref()) {
                (Some(bundle), Some(tip)) => bundle.collateral_base.with_view(tip, |view| {
                    f(&TradingNativeView {
                        positions,
                        orders,
                        collateral: Some(view),
                    })
                }),
                _ => f(&TradingNativeView {
                    positions,
                    orders,
                    collateral: None,
                }),
            }
        };
        let with_orders = |positions: Option<&NativeStateView<'_>>| match (
            self.order.as_ref(),
            order_tip.as_ref(),
        ) {
            (Some(bundle), Some(tip)) => bundle
                .order_base
                .with_view(tip, |view| with_collateral(positions, Some(view))),
            _ => with_collateral(positions, None),
        };
        match (self.position.as_ref(), position_tip.as_ref()) {
            (Some(bundle), Some(tip)) => bundle
                .position_base
                .with_view(tip, |view| with_orders(Some(view))),
            _ => with_orders(None),
        }
    }
}
