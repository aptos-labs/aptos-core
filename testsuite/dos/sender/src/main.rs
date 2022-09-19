// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use clap::Parser;
use event_listener::Event;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Duration;
//use quanta::Clock;

static STOP: AtomicBool = AtomicBool::new(false);
static BYTESIN: AtomicU64 = AtomicU64::new(0);
#[cfg(not(target_arch = "x86_64"))]
const DEFAULT_ADDR: &str = "127.0.0.1:6142";
#[cfg(target_arch = "x86_64")]
const DEFAULT_ADDR: &str = "10.100.62.151:6180";

/// TCP sender
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// listner ip:port
    #[clap(short, long, value_parser)]
    addr: Option<String>,

    #[clap(short, long, value_parser, default_value_t = false)]
    load: bool,

    #[clap(short, long, value_parser, default_value_t = false)]
    stream: bool,

    #[clap(short, long, value_parser, default_value_t = 0)]
    cap: u64,

    #[clap(short, long, value_parser, default_value_t = 0)]
    burst: u16,

    #[clap(short, long, value_parser, default_value_t = 128.0)]
    hertz: f64,
}

async fn stream_rx(addr: Arc<String>, cap: u64) -> io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    let mut stream = TcpStream::connect(&*addr).await?;
    let mut total = 0;

    stream.write_all(b"Gimme some Bytes World!\n").await?;

    loop {
        let n = stream.read_buf(&mut buf).await?;
        if n == 0 {
            println!("RX Stream out");
            return Ok::<(), tokio::io::Error>(());
        }
        total += n;
        if total >= cap as usize {
            println!("RX Stream Done {total}/{cap}");
            return Ok::<(), tokio::io::Error>(());
        }

        buf.clear();
        BYTESIN.fetch_add(n as u64, Ordering::Relaxed);
    }
}

async fn burst_target(addr: Arc<String>, burst: u16) {
    let inner_counter = Arc::new(AtomicU64::new(1));
    let ack_counter = Arc::new(AtomicU64::new(0));
    let done = Arc::new(Event::new());
    let listner = done.listen();

    for i in 0..burst {
        let counter = inner_counter.clone();
        let ack = ack_counter.clone();
        let daddr = addr.clone();
        let done = done.clone();

        tokio::spawn(async move {
            let stream = TcpStream::connect(&*daddr).await;
            if let Err(e) = stream {
                println!("{i} : Error {e}");
                let curr = counter.fetch_add(1, Ordering::Relaxed);

                if curr == burst.into() {
                    done.notify(1);
                }
                return Err(e);
            }
            let mut stream = stream?;
            let mut buf = BytesMut::with_capacity(4096);

            _ = stream.set_nodelay(true);
            //println!("{:?} Connected to {:?}", stream.local_addr(), stream.peer_addr());
            stream.write_all(b"Hello World!\n").await?;
            let n = stream.read_buf(&mut buf).await?;
            let curr = counter.fetch_add(1, Ordering::Relaxed);

            if n > 0 {
                //this is ugly...
                if n < 16 {
                    println!(
                        "{curr}/{}:{}: Got: {:?}",
                        burst,
                        ack.fetch_add(1, Ordering::Relaxed),
                        String::from_utf8((&buf[..n]).to_vec()).unwrap()
                    );
                } else {
                    println!(
                        "{curr}/{}:{}: Received: {n} bytes",
                        burst,
                        ack.fetch_add(1, Ordering::Relaxed)
                    );
                }
            } /*else {
                  println!("{curr}/{}:{}: Silent", burst, ack.load(Ordering::Relaxed));
              }*/

            if curr == burst.into() {
                println!("{} cya cowboy!\n", curr);
                done.notify(1);
            }

            Ok::<_, tokio::io::Error>(())
        });
        tokio::task::yield_now().await;
    }

    listner.await;
}

async fn load_target(addr: Arc<String>, step: u64) {
    let inner_counter = Arc::new(AtomicU64::new(0));
    let inner_err_counter = Arc::new(AtomicU64::new(0));
    let inner_conn_counter = Arc::new(AtomicU64::new(0));

    let counter = inner_counter.clone();
    let con_at = inner_conn_counter.clone();
    let con_err = inner_err_counter.clone();

    //let BYTESIN = BytesIN.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            let conn = counter.load(Ordering::Relaxed);
            let attempt = con_at.load(Ordering::Relaxed);
            let err = con_err.load(Ordering::Relaxed);

            let rx_bytes = BYTESIN.load(Ordering::Relaxed);

            println!(
                "Approx: {attempt}/{} connections in last sec, {err} errors: BytesIN {}",
                conn, rx_bytes,
            );

            counter.fetch_sub(conn, Ordering::SeqCst);
            con_at.fetch_sub(attempt, Ordering::SeqCst);
            con_err.fetch_sub(err, Ordering::SeqCst);
            BYTESIN.fetch_sub(rx_bytes, Ordering::SeqCst);
        }
    });

    while !STOP.load(Ordering::Relaxed) {
        let daddr = addr.clone();
        let con_at = inner_conn_counter.clone();
        let con_err = inner_err_counter.clone();

        if step > 0 {
            tokio::time::sleep(Duration::from_millis(step)).await;
        }
        inner_counter.fetch_add(1, Ordering::Relaxed);
        tokio::spawn(async move {
            let stream = TcpStream::connect(&*daddr).await;
            if let Err(_e) = stream {
                con_err.fetch_add(1, Ordering::Relaxed);
            } else {
                con_at.fetch_add(1, Ordering::Relaxed);
            }
        });
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let addr = {
        if let Some(addr) = args.addr {
            Arc::new(addr)
        } else {
            Arc::new(DEFAULT_ADDR.to_string())
        }
    };

    let mut _stream = TcpStream::connect(&*addr.clone()).await?;

    if args.burst > 0 {
        burst_target(addr.clone(), args.burst).await;
    }

    if args.stream {
        tokio::spawn(stream_rx(addr.clone(), args.cap));
    }

    if args.load {
        let mut step = 0.0;
        if args.hertz > 0.0 {
            step = 1000.0 / args.hertz;
        }

        load_target(addr.clone(), step as u64).await;
    }

    Ok(())
}
