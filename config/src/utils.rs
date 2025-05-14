// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::NodeConfig;
use aptos_crypto::_once_cell::sync::Lazy;
use aptos_logger::prelude::*;
use aptos_types::{
    network_address::{NetworkAddress, Protocol},
    transaction::Transaction,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use get_if_addrs::get_if_addrs;
use rand::{seq::SliceRandom, SeedableRng};
use std::{
    env, fs,
    fs::{File, OpenOptions},
    io::Seek,
    net::{TcpListener, TcpStream},
    ops::Range,
    thread,
    time::Duration,
};

const MAX_PORT_RETRIES: u16 = 1000;
// Using non-ephemeral ports, to avoid conflicts with OS-selected ports (i.e., bind on port 0)
const UNIQUE_PORT_RANGE: Range<u16> = 10000..30000;
// Consistent seed across processes
const PORT_SEED: [u8; 32] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7,
];
// See https://nexte.st/book/env-vars.html#environment-variables-nextest-sets
static NEXTEST_RUN_ID: Lazy<Option<String>> = Lazy::new(|| {
    if let Ok(run_id) = env::var("NEXTEST_RUN_ID") {
        Some(run_id)
    } else {
        None
    }
});
static PORT_VECTOR: Lazy<Vec<u16>> = Lazy::new(|| {
    let mut ports: Vec<_> = UNIQUE_PORT_RANGE.collect();
    let mut rng = rand::rngs::StdRng::from_seed(PORT_SEED);
    ports.shuffle(&mut rng);
    ports
});

struct PortCounterFiles {
    counter_file: File,
    _lock_file: File,
}

impl PortCounterFiles {
    fn new(counter_file: File, lock_file: File) -> Self {
        Self {
            counter_file,
            _lock_file: lock_file,
        }
    }
}

impl Drop for PortCounterFiles {
    fn drop(&mut self) {
        fs::remove_file(lock_path()).unwrap();
    }
}

pub fn get_available_port() -> u16 {
    if NEXTEST_RUN_ID.is_some() {
        get_unique_port()
    } else {
        get_random_port()
    }
}

/// Return an ephemeral, available port. On unix systems, the port returned will be in the
/// TIME_WAIT state ensuring that the OS won't hand out this port for some grace period.
/// Callers should be able to bind to this port given they use SO_REUSEADDR.
fn get_random_port() -> u16 {
    for _ in 0..MAX_PORT_RETRIES {
        if let Ok(port) = try_bind(None) {
            return port;
        }
    }

    panic!("Error: could not find an available port");
}

fn try_bind(port: Option<u16>) -> ::std::io::Result<u16> {
    // Use the provided port or 0 to request a random available port from the OS
    let port = port.unwrap_or_default(); // Defaults to 0
    let listener = TcpListener::bind(("localhost", port))?;
    let addr = listener.local_addr()?;

    // Create and accept a connection (which we'll promptly drop) in order to force the port
    // into the TIME_WAIT state, ensuring that the port will be reserved from some limited
    // amount of time (roughly 60s on some Linux systems)
    let _sender = TcpStream::connect(addr)?;
    let _incoming = listener.accept()?;

    Ok(addr.port())
}

fn lock_path() -> String {
    format!(
        "/tmp/aptos-port-counter.{}.lock",
        &NEXTEST_RUN_ID.clone().unwrap()
    )
}

fn counter_path() -> String {
    format!(
        "/tmp/aptos-port-counter.{}",
        &NEXTEST_RUN_ID.clone().unwrap()
    )
}

/// We use the filesystem to bind to unique ports for cargo nextest.
/// cargo nextest runs tests concurrently in different processes. We have observed that using
/// a simple bind(0) results in flaky tests when nodes are restarted within tests; likely the OS
/// is prioritizing recently released ports.
fn get_unique_port() -> u16 {
    let mut port_counter_files = open_counter_file();
    let global_counter = match port_counter_files.counter_file.read_u16::<BigEndian>() {
        Ok(counter) => {
            if counter as usize >= PORT_VECTOR.len() {
                0
            } else {
                counter
            }
        },
        Err(_) => {
            warn!(
                "Unable to read port counter from file {}, starting from 0",
                counter_path()
            );
            0
        },
    };
    let (port, updated_counter) = bind_port_from_counter(global_counter);

    port_counter_files.counter_file.set_len(0).unwrap();
    port_counter_files.counter_file.rewind().unwrap();
    port_counter_files
        .counter_file
        .write_u16::<BigEndian>(updated_counter)
        .unwrap();

    port
}

fn open_counter_file() -> PortCounterFiles {
    for i in 0..100 {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(lock_path())
        {
            Ok(lock_file) => match OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(counter_path())
            {
                Ok(counter_file) => return PortCounterFiles::new(counter_file, lock_file),
                Err(_) => {
                    panic!("Could not read {}", counter_path());
                },
            },
            Err(_) => {
                info!("Lock could not be acquired, attempt {}", i);
                thread::sleep(Duration::from_millis(100));
            },
        }
    }
    panic!("Could not acquire lock to: {}", lock_path());
}

fn bind_port_from_counter(mut counter: u16) -> (u16, u16) {
    for attempt in 0..MAX_PORT_RETRIES {
        let port = PORT_VECTOR[counter as usize];
        counter += 1;
        if counter as usize == PORT_VECTOR.len() {
            counter = 0;
        }

        match try_bind(Some(port)) {
            Ok(port) => {
                return (port, counter);
            },
            Err(_) => {
                info!(
                    "Conflicting port: {}, on count {} and attempt {}",
                    port, counter, attempt
                );
                continue;
            },
        }
    }

    panic!(
        "Error: could not find an available port. Counter: {}",
        counter
    );
}

/// Extracts one local non-loopback IP address, if one exists. Otherwise returns None.
pub fn get_local_ip() -> Option<NetworkAddress> {
    get_if_addrs().ok().and_then(|if_addrs| {
        if_addrs
            .iter()
            .find(|if_addr| !if_addr.is_loopback())
            .map(|if_addr| NetworkAddress::from(Protocol::from(if_addr.ip())))
    })
}

pub fn get_available_port_in_multiaddr(is_ipv4: bool) -> NetworkAddress {
    let ip_proto = if is_ipv4 {
        Protocol::Ip4("0.0.0.0".parse().unwrap())
    } else {
        Protocol::Ip6("::1".parse().unwrap())
    };
    NetworkAddress::from_protocols(vec![ip_proto, Protocol::Tcp(get_available_port())]).unwrap()
}

pub fn get_genesis_txn(config: &NodeConfig) -> Option<&Transaction> {
    config.execution.genesis.as_ref()
}
