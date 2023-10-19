// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Instant;
use clap::Parser;
use aptos_executor_service::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_types::access_path::AccessPath;
use aptos_types::state_store::state_key::StateKey;

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
    let num_iterations = opt.num_iterations;
    let num_state_keys = opt.num_state_keys;
    let server_addr = opt.self_address;
    let remote_server_addr = opt.remote_address;

    let mut network_controller =
        NetworkController::new("perf_benchmark".to_string(), server_addr, 1000);
    let sender = network_controller.create_outbound_channel(remote_server_addr, "remote_kv_request".to_string());
    let receiver = network_controller.create_inbound_channel("remote_kv_response".to_string());

    // start server and outbound thread
    network_controller.start();
    sleep(std::time::Duration::from_millis(100));

    let mut state_keys = Vec::new();
    for i in 1000..num_state_keys+1000 {
        let key = StateKey::access_path(AccessPath::new(i.to_string().parse().unwrap(), vec![7, 2, 3]));
        state_keys.push(key);
    }
    let request = RemoteKVRequest::new(0, state_keys);

    let mut serialization_time = 0.0;
    let mut deserialization_time = 0.0;
    let mut msg_tx_rx_time = 0.0;
    for _ in 0..num_iterations {
        let mut start_time = Instant::now();
        let request_message = bcs::to_bytes(&request).unwrap();
        serialization_time += start_time.elapsed().as_secs_f64();
        // send the message to remote side

        start_time = Instant::now();
        sender.send(Message::new(request_message)).unwrap();
        let received_message = receiver.recv().unwrap();
        msg_tx_rx_time += start_time.elapsed().as_secs_f64();

        start_time = Instant::now();
        let _dummy: RemoteKVResponse = bcs::from_bytes(&received_message.data).unwrap();
        deserialization_time += start_time.elapsed().as_secs_f64();
    }

    println!("Total durations for {} requests with {} keys in each request: req bcs ser {} s; resp bcs deser {} s; tx {} s",
             num_iterations, num_state_keys, serialization_time, deserialization_time, msg_tx_rx_time);
    sleep(std::time::Duration::from_millis(100));
}
