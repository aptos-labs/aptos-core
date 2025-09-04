// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Error};
use velor_consensus::{
    persistent_liveness_storage::PersistentLivenessStorage,
    quorum_store::quorum_store_db::QuorumStoreStorage, util::db_tool::extract_txns_from_block,
};
use velor_crypto::HashValue;
use velor_logger::info;
use velor_system_utils::utils::{reply_with, reply_with_status, spawn_blocking};
use velor_types::transaction::Transaction;
use http::header::{HeaderValue, CONTENT_LENGTH};
use hyper::{Body, Request, Response, StatusCode};
use std::{collections::HashMap, sync::Arc};

pub async fn handle_dump_consensus_db_request(
    _req: Request<Body>,
    consensus_db: Arc<dyn PersistentLivenessStorage>,
) -> hyper::Result<Response<Body>> {
    info!("Dumping consensus db.");

    match spawn_blocking(move || dump_consensus_db(consensus_db.as_ref())).await {
        Ok(result) => {
            info!("Finished dumping consensus db.");
            let headers: Vec<(_, HeaderValue)> =
                vec![(CONTENT_LENGTH, HeaderValue::from(result.len()))];
            Ok(reply_with(headers, result))
        },
        Err(e) => {
            info!("Failed to dump consensus db: {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        },
    }
}

pub async fn handle_dump_quorum_store_db_request(
    req: Request<Body>,
    quorum_store_db: Arc<dyn QuorumStoreStorage>,
) -> hyper::Result<Response<Body>> {
    let query = req.uri().query().unwrap_or("");
    let query_pairs: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).collect();

    let digest: Option<HashValue> = match query_pairs.get("digest") {
        Some(val) => match val.parse() {
            Ok(val) => Some(val),
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => None,
    };

    info!("Dumping quorum store db.");

    match spawn_blocking(move || dump_quorum_store_db(quorum_store_db.as_ref(), digest)).await {
        Ok(result) => {
            info!("Finished dumping quorum store db.");
            let headers: Vec<(_, HeaderValue)> =
                vec![(CONTENT_LENGTH, HeaderValue::from(result.len()))];
            Ok(reply_with(headers, result))
        },
        Err(e) => {
            info!("Failed to dump quorum store db: {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        },
    }
}

pub async fn handle_dump_block_request(
    req: Request<Body>,
    consensus_db: Arc<dyn PersistentLivenessStorage>,
    quorum_store_db: Arc<dyn QuorumStoreStorage>,
) -> hyper::Result<Response<Body>> {
    let query = req.uri().query().unwrap_or("");
    let query_pairs: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).collect();

    let block_id: Option<HashValue> = match query_pairs.get("block_id") {
        Some(val) => match val.parse() {
            Ok(val) => Some(val),
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => None,
    };

    // TODO(grao): I'm lazy, only support this through query parameters, let me know if this need
    // to be done through header.
    let bcs: bool = match query_pairs.get("bcs") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => false,
    };

    if let Some(block_id) = block_id {
        info!("Dumping block ({block_id:?}).");
    } else {
        info!("Dumping all blocks.");
    }

    match spawn_blocking(move || {
        if bcs {
            dump_blocks_bcs(consensus_db.as_ref(), quorum_store_db.as_ref(), block_id)
                .map(Into::<Body>::into)
        } else {
            dump_blocks(consensus_db.as_ref(), quorum_store_db.as_ref(), block_id).map(Into::into)
        }
    })
    .await
    {
        Ok(result) => {
            info!("Finished dumping block(s).");
            Ok(reply_with(vec![], result))
        },
        Err(e) => {
            info!("Failed to dump block(s): {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        },
    }
}

fn dump_consensus_db(consensus_db: &dyn PersistentLivenessStorage) -> anyhow::Result<String> {
    let mut body = String::new();

    let (last_vote, highest_tc, consensus_blocks, consensus_qcs) =
        consensus_db.consensus_db().get_data()?;

    body.push_str(&format!("Last vote: \n{last_vote:?}\n\n"));
    body.push_str(&format!("Highest tc: \n{highest_tc:?}\n\n"));
    body.push_str("Blocks: \n");
    for block in consensus_blocks {
        body.push_str(&format!(
            "[id: {:?}, author: {:?}, epoch: {}, round: {:02}, parent_id: {:?}, timestamp: {}, payload: {:?}]\n\n",
            block.id(),
            block.author(),
            block.epoch(),
            block.round(),
            block.parent_id(),
            block.timestamp_usecs(),
            block.payload(),
        ));
    }
    body.push_str("QCs: \n");
    for qc in consensus_qcs {
        body.push_str(&format!("{qc:?}\n\n"));
    }
    Ok(body)
}

fn dump_quorum_store_db(
    quorum_store_db: &dyn QuorumStoreStorage,
    digest: Option<HashValue>,
) -> anyhow::Result<String> {
    let mut body = String::new();

    if let Some(digest) = digest {
        body.push_str(&format!("{digest:?}:\n"));
        body.push_str(&format!(
            "{:?}",
            quorum_store_db.get_batch(&digest).map_err(Error::msg)?
        ));
    } else {
        for (digest, _batch) in quorum_store_db.get_all_batches()? {
            body.push_str(&format!("{digest:?}:\n"));
        }
    }

    Ok(body)
}

fn dump_blocks(
    consensus_db: &dyn PersistentLivenessStorage,
    quorum_store_db: &dyn QuorumStoreStorage,
    block_id: Option<HashValue>,
) -> anyhow::Result<String> {
    let mut body = String::new();

    let all_batches = quorum_store_db.get_all_batches()?;

    let (_, _, blocks, _) = consensus_db.consensus_db().get_data()?;

    for block in blocks {
        let id = block.id();
        if block_id.is_none() || id == block_id.unwrap() {
            body.push_str(&format!("Block ({id:?}): \n\n"));
            match extract_txns_from_block(&block, &all_batches) {
                Ok(txns) => {
                    body.push_str(&format!("{txns:?}"));
                },
                Err(e) => {
                    body.push_str(&format!("Not available: {e:?}"));
                },
            };
            body.push_str("\n\n");
        }
    }

    if body.is_empty() {
        if let Some(block_id) = block_id {
            body.push_str(&format!("Done, block ({block_id:?}) is not found."));
        } else {
            body.push_str("Done, no block is found.");
        }
    }

    Ok(body)
}

fn dump_blocks_bcs(
    consensus_db: &dyn PersistentLivenessStorage,
    quorum_store_db: &dyn QuorumStoreStorage,
    block_id: Option<HashValue>,
) -> anyhow::Result<Vec<u8>> {
    let all_batches = quorum_store_db.get_all_batches()?;

    let (_, _, blocks, _) = consensus_db.consensus_db().get_data()?;

    let mut all_txns = Vec::new();
    for block in blocks {
        let id = block.id();
        if block_id.is_none() || id == block_id.unwrap() {
            match extract_txns_from_block(&block, &all_batches) {
                Ok(txns) => {
                    all_txns.extend(txns.into_iter().cloned().map(Transaction::UserTransaction));
                },
                Err(e) => bail!("Failed to extract txns from block ({id:?}): {e:?}."),
            };
        }
    }

    bcs::to_bytes(&all_txns).map_err(Error::msg)
}
