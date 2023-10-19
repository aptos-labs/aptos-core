// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Instant;
use bytes::Bytes;
use clap::Parser;
use aptos_secure_net::network_controller::{Message, NetworkController};
use rand::Rng;
use aptos_executor_service::{RemoteKVRequest, RemoteKVResponse};
use aptos_types::state_store::state_value::StateValue;

#[derive(Parser, Debug)]
struct Opt {
    #[clap(long, default_value = "200000")]
    num_state_keys: usize,

    #[clap(long, default_value = "10")]
    num_iterations: usize,

    #[clap(long)]
    self_address: SocketAddr,

    #[clap(long)]
    remote_address: SocketAddr,
}

fn main() {
    let opt = Opt::parse();
    let num_state_keys = opt.num_state_keys;
    let server_addr = opt.self_address;
    let remote_server_addr = opt.remote_address;

    let mut network_controller =
        NetworkController::new("perf_benchmark".to_string(), server_addr, 1000);
    let sender = network_controller.create_outbound_channel(remote_server_addr, "remote_kv_response".to_string());
    let receiver = network_controller.create_inbound_channel("remote_kv_request".to_string());

    // start server and outbound thread
    network_controller.start();

    // Generate random bytes of the given size
    let bytes_per_key_value = 100;
    let mut rng = rand::thread_rng();
    let random_bytes: Bytes = (0..bytes_per_key_value)
        .map(|_| rng.gen::<u8>())
        .collect::<Vec<_>>()
        .into();
    let state_values: Vec<StateValue> = (0..num_state_keys)
        .map(|_| {
            StateValue::new_legacy(random_bytes.clone())
        })
        .collect();

    let mut serialization_time = 0.0;
    let mut deserialization_time = 0.0;
    let mut total_resp_processing_time = 0.0;
    let mut num_messages = 0;

    while num_messages < opt.num_iterations {
        let received_message = receiver.recv().unwrap();
        let start_time = Instant::now();

        let req: RemoteKVRequest = bcs::from_bytes(&received_message.data).unwrap();
        deserialization_time += start_time.elapsed().as_secs_f64();
        let (_, state_keys) = req.into();
        assert_eq!(state_keys.len(), num_state_keys);
        let resp = (0..num_state_keys)
            .map(|i| {
                (state_keys[i].clone(), Some(state_values[i].clone()))
            })
            .collect();
        let kv_resp = RemoteKVResponse::new(resp);

        let resp_ser_start_time = Instant::now();
        let resp_message = bcs::to_bytes(&kv_resp).unwrap();
        serialization_time += resp_ser_start_time.elapsed().as_secs_f64();

        num_messages += 1;
        total_resp_processing_time += start_time.elapsed().as_secs_f64();
        // send back same message
        sender.send(Message::new(resp_message)).unwrap();
    }
    println!("Total durations for {} requests with {} keys in each request: req bcs deser {} s; resp bcs ser {} s; total resp processing {} s",
             num_messages, num_state_keys, deserialization_time, serialization_time, total_resp_processing_time);
    sleep(std::time::Duration::from_millis(100));
}
