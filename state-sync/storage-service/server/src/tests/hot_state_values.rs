// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::{mock, mock::MockClient, utils};
use aptos_config::config::StorageServiceConfig;
use aptos_crypto::HashValue;
use aptos_storage_service_types::{
    responses::{DataResponse, StorageServiceResponse},
    StorageServiceError,
};
use aptos_types::{
    proof::definition::SparseMerkleRangeProof,
    state_store::{
        hot_state::{HotStateValue, HotStateValueChunkWithProof},
        state_key::StateKey,
        state_value::StateValue,
    },
};
use bytes::Bytes;
use claims::assert_matches;
use mockall::predicate::eq;

#[tokio::test]
async fn test_get_hot_states_with_proof_and_cache() {
    let version = 101;
    let start_index = 100;
    let chunk_size = 10;
    let raw_values = create_hot_state_values(chunk_size, 10, version);

    let mut db_reader = mock::create_mock_db_reader();
    expect_get_hot_state_values_with_proof(
        &mut db_reader,
        version,
        start_index,
        chunk_size,
        raw_values.clone(),
        2, // One storage fetch for each compression setting.
    );

    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), None);
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    for use_compression in [false, true] {
        // The second identical request should be served from the response cache.
        for _ in 0..2 {
            let response = utils::get_hot_state_values_with_proof(
                &mut mock_client,
                version,
                start_index,
                start_index + chunk_size - 1,
                use_compression,
            )
            .await
            .unwrap();
            assert_eq!(response.is_compressed(), use_compression);
            assert_hot_state_response(response, start_index, &raw_values);
        }
    }
}

#[tokio::test]
async fn test_get_hot_states_with_proof_chunk_limit() {
    let config = StorageServiceConfig::default();
    let version = 101;
    let start_index = 100;
    let requested_chunk_size = config.max_state_chunk_size + 10;
    let raw_values = create_hot_state_values(config.max_state_chunk_size, 1, version);

    let mut db_reader = mock::create_mock_db_reader();
    expect_get_hot_state_values_with_proof(
        &mut db_reader,
        version,
        start_index,
        config.max_state_chunk_size,
        raw_values.clone(),
        1,
    );

    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), Some(config));
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    let response = utils::get_hot_state_values_with_proof(
        &mut mock_client,
        version,
        start_index,
        start_index + requested_chunk_size - 1,
        false,
    )
    .await
    .unwrap();
    assert_hot_state_response(response, start_index, &raw_values);
}

#[tokio::test]
async fn test_get_hot_states_with_proof_network_limit() {
    let network_limit_bytes = 2500;
    let version = 101;
    let start_index = 100;
    let chunk_size = 10;
    let raw_values = create_hot_state_values(chunk_size, 1000, version);
    let config = StorageServiceConfig {
        max_network_chunk_bytes: network_limit_bytes,
        ..Default::default()
    };

    let mut db_reader = mock::create_mock_db_reader();
    expect_get_hot_state_values_with_proof(
        &mut db_reader,
        version,
        start_index,
        chunk_size,
        raw_values,
        1,
    );

    let (mut mock_client, mut service, _, _, _) = MockClient::new(Some(db_reader), Some(config));
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    let response = utils::get_hot_state_values_with_proof(
        &mut mock_client,
        version,
        start_index,
        start_index + chunk_size - 1,
        false,
    )
    .await
    .unwrap();
    let response_size = bcs::serialized_size(&response).unwrap() as u64;
    let DataResponse::HotStateValueChunkWithProof(chunk) = response.get_data_response().unwrap()
    else {
        panic!("expected hot state values with proof response")
    };
    assert!(chunk.raw_values.len() < chunk_size as usize);
    if response_size > network_limit_bytes {
        assert_eq!(chunk.raw_values.len(), 1);
    }
}

#[tokio::test]
async fn test_get_hot_states_with_proof_invalid_range() {
    let version = 101;
    let (mut mock_client, mut service, _, _, _) = MockClient::new(None, None);
    utils::update_storage_server_summary(&mut service, version, 10);
    tokio::spawn(service.start());

    let response =
        utils::get_hot_state_values_with_proof(&mut mock_client, version, 100, 99, false)
            .await
            .unwrap_err();
    assert_matches!(response, StorageServiceError::InvalidRequest(_));
}

fn create_hot_state_values(
    num_values: u64,
    value_size: usize,
    hot_since_version: u64,
) -> Vec<(StateKey, HotStateValue)> {
    (0..num_values)
        .map(|index| {
            let state_key = StateKey::raw(&index.to_le_bytes());
            let state_value = StateValue::new_legacy(Bytes::from(vec![index as u8; value_size]));
            (
                state_key,
                HotStateValue::new(Some(state_value), hot_since_version),
            )
        })
        .collect()
}

fn expect_get_hot_state_values_with_proof(
    db_reader: &mut mock::MockDatabaseReader,
    version: u64,
    start_index: u64,
    chunk_size: u64,
    raw_values: Vec<(StateKey, HotStateValue)>,
    times: usize,
) {
    let iterator_values = raw_values.clone();
    db_reader
        .expect_get_hot_state_value_chunk_iter()
        .times(times)
        .with(
            eq(version),
            eq(start_index as usize),
            eq(chunk_size as usize),
        )
        .returning(move |_, _, _| Ok(Box::new(iterator_values.clone().into_iter().map(Ok))));

    db_reader
        .expect_get_hot_state_value_chunk_proof()
        .times(times)
        .with(
            eq(version),
            eq(start_index as usize),
            mockall::predicate::always(),
        )
        .returning(move |_, first_index, raw_values| {
            let last_index = first_index + raw_values.len() - 1;
            Ok(HotStateValueChunkWithProof {
                first_index: first_index as u64,
                last_index: last_index as u64,
                first_key: HashValue::random(),
                last_key: HashValue::random(),
                raw_values,
                proof: SparseMerkleRangeProof::new(vec![]),
                root_hash: HashValue::random(),
            })
        });
}

fn assert_hot_state_response(
    response: StorageServiceResponse,
    start_index: u64,
    expected_raw_values: &[(StateKey, HotStateValue)],
) {
    let DataResponse::HotStateValueChunkWithProof(chunk) = response.get_data_response().unwrap()
    else {
        panic!("expected hot state values with proof response")
    };
    assert_eq!(chunk.first_index, start_index);
    assert_eq!(chunk.raw_values, expected_raw_values);
}
