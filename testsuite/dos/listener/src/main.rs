// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use clap::Parser;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::time::Duration;

static STOP: AtomicBool = AtomicBool::new(false);
#[cfg(not(target_arch = "x86_64"))]
const DEFAULT_ADDR: &str = "127.0.0.1:6142";
#[cfg(target_arch = "x86_64")]
const DEFAULT_ADDR: &str = "0.0.0.0:6181";

/// TCP listener
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// listener ip:port
    #[clap(short, long, value_parser)]
    addr: Option<String>,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let args = Args::parse();

    let listener = {
        if let Some(addr) = args.addr {
            println!("listening on {addr}");
            TcpListener::bind(addr).await?
        } else {
            println!("listening on: {} (default)", DEFAULT_ADDR);
            TcpListener::bind(DEFAULT_ADDR).await?
        }
    };

    let inner_counter = Arc::new(AtomicU64::new(0));
    let counter = inner_counter.clone();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            let conn = counter.load(Ordering::Relaxed);

            if conn > 0 {
                println!("{conn} connections in last epoch\n");
                counter.store(0, Ordering::SeqCst);
            }
        }
    });

    while !STOP.load(Ordering::Relaxed) {
        let mut buffer = BytesMut::with_capacity(4096);
        let (mut socket, _) = listener.accept().await?;

        let load = inner_counter.fetch_add(1, Ordering::Relaxed);

        if load < 8 {
            tokio::spawn(async move {
                println!("connection from {:?} - {}", socket, load);

                for _ in 0..128 {
                    //let _ = socket.ready(Interest::READABLE).await?;
                    match socket.try_read_buf(&mut buffer) {
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            tokio::task::yield_now().await;
                        }
                        Err(e) => {
                            return Err(e);
                        }
                        Ok(0) => break,
                        Ok(n) => {
                            if &buffer[..13] == b"Hello World!\n" {
                                socket.write_all(b"Hello World!\n").await?;
                            } else {
                                let mut total = 0;
                                loop {
                                    let mut buffer = BytesMut::with_capacity(4096);
                                    unsafe {
                                        buffer.set_len(4096);
                                    }
                                    let m = socket.write_buf(&mut buffer).await?;
                                    if m == 0 {
                                        println!("{n} In. Sent {total} Bytes");
                                        break;
                                    } else {
                                        total += m;
                                    }
                                }
                            }
                        }
                    }
                }

                Ok::<_, tokio::io::Error>(())
            });
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}
