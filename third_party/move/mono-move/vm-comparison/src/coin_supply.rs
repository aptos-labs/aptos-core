// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Rewrites parallelizable coin supply into its sequential form.

use anyhow::Result;
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::{
    account_config::{CoinInfoResource, IntegerResource},
    state_store::{state_key::StateKey, state_value::StateValue, TStateView},
    AptosCoinType, CoinType,
};

type AptosCoinInfo = CoinInfoResource<AptosCoinType>;

/// `AptosCoin` is the only coin that ever had a parallelizable supply.
fn coin_info_key() -> Result<StateKey> {
    StateKey::resource_typed::<AptosCoinInfo>(&AptosCoinType::coin_info_address())
}

/// Moves `AptosCoin`'s supply off the aggregator and into the plain `Integer`
/// counter, folding in the aggregator's current value.
pub fn sequentialize_coin_supply(state: &InMemoryStateStore) -> Result<()> {
    let key = coin_info_key()?;
    let Some(value) = state.get_state_value(&key)? else {
        return Ok(());
    };

    let mut coin_info = bcs::from_bytes::<AptosCoinInfo>(value.bytes())
        .expect("AptosCoin's CoinInfo must deserialize");
    let Some(supply) = coin_info.supply_mut().as_mut() else {
        return Ok(());
    };
    let Some(aggregator) = supply.aggregator.take() else {
        return Ok(());
    };

    // A capture records what the transaction read, so the aggregator's value
    // can be absent. Guessing a supply would change what the transaction
    // computes.
    let Some(current) = state.get_state_value(&aggregator.state_key())? else {
        return Ok(());
    };
    let current =
        bcs::from_bytes::<u128>(current.bytes()).expect("AptosCoin's supply must deserialize");
    supply.integer = Some(IntegerResource::new(current, aggregator.limit()));
    state.set_state_value(
        key,
        StateValue::new_legacy(bcs::to_bytes(&coin_info)?.into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_coin_info(supply: u128, with_value: bool) -> InMemoryStateStore {
        let coin_info = AptosCoinInfo::random(u128::MAX);
        let state = InMemoryStateStore::new();

        let bytes = bcs::to_bytes(&coin_info).unwrap();
        state
            .set_state_value(coin_info_key().unwrap(), StateValue::new_legacy(bytes.into()))
            .unwrap();
        if with_value {
            let bytes = bcs::to_bytes(&supply).unwrap();
            state
                .set_state_value(
                    coin_info.supply_aggregator_state_key(),
                    StateValue::new_legacy(bytes.into()),
                )
                .unwrap();
        }
        state
    }

    #[test]
    fn test_supply_is_folded_onto_the_integer_branch() {
        let state = state_with_coin_info(12345, true);
        sequentialize_coin_supply(&state).unwrap();

        let key = coin_info_key().unwrap();
        let bytes = state.get_state_value(&key).unwrap().unwrap();
        let coin_info = bcs::from_bytes::<AptosCoinInfo>(bytes.bytes()).unwrap();
        let supply = coin_info.supply().as_ref().unwrap();
        assert!(supply.aggregator.is_none());
        assert_eq!(supply.integer.as_ref().unwrap().value, 12345);
    }

    #[test]
    fn test_coin_info_without_its_aggregator_value_is_left_alone() {
        let state = state_with_coin_info(12345, false);
        let key = coin_info_key().unwrap();
        let before = state.get_state_value(&key).unwrap().unwrap();

        sequentialize_coin_supply(&state).unwrap();

        assert_eq!(state.get_state_value(&key).unwrap().unwrap(), before);
    }
}
