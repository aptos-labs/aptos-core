// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::info;
use velor_mempool::{MempoolClientRequest, MempoolClientSender};
use velor_system_utils::utils::{reply_with, reply_with_status};
use velor_types::account_address::AccountAddress;
use futures_channel::oneshot::Canceled;
use http::{Request, Response, StatusCode};
use hyper::Body;

pub async fn mempool_handle_parking_lot_address_request(
    _req: Request<Body>,
    mempool_client_sender: MempoolClientSender,
) -> hyper::Result<Response<Body>> {
    match get_parking_lot_addresses(mempool_client_sender).await {
        Ok(addresses) => {
            info!("Finished getting parking lot addresses from mempool.");
            match bcs::to_bytes(&addresses) {
                Ok(addresses) => Ok(reply_with(vec![], addresses)),
                Err(e) => {
                    info!("Failed to bcs serialize parking lot addresses from mempool: {e:?}");
                    Ok(reply_with_status(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        e.to_string(),
                    ))
                },
            }
        },
        Err(e) => {
            info!("Failed to get parking lot addresses from mempool: {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        },
    }
}

async fn get_parking_lot_addresses(
    mempool_client_sender: MempoolClientSender,
) -> Result<Vec<(AccountAddress, u64)>, Canceled> {
    let (sender, receiver) = futures_channel::oneshot::channel();

    match mempool_client_sender
        .clone()
        .try_send(MempoolClientRequest::GetAddressesFromParkingLot(sender))
    {
        Ok(_) => receiver.await,
        Err(e) => {
            info!("Failed to send request for GetAddressesFromParkingLot: {e:?}");
            Err(Canceled)
        },
    }
}
