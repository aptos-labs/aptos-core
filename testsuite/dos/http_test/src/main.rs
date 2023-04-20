// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Router};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!\n" }));

    let stats = Router::new().route("/", get(|| async { "Hello, World!\n" }));

    let h1 = tokio::spawn(async {
        axum::Server::bind(&"0.0.0.0:80".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    let h2 = tokio::spawn(async {
        axum::Server::bind(&"0.0.0.0:9101".parse().unwrap())
            .serve(stats.into_make_service())
            .await
            .unwrap();
    });

    let h3 = tokio::spawn(async {
        let addr = "0.0.0.0:6180";
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);
        let (mut socket, _) = listener.accept().await.unwrap();
        println!("Connection from {:#?}", socket);
        let mut buf = vec![0; 1024];

        let n = socket
            .read(&mut buf)
            .await
            .expect("failed to read data from socket");

        if n == 0 {
            return;
        }

        socket
            .write_all(&buf[0..n])
            .await
            .expect("failed to write data to socket");
    });

    let h4 = tokio::spawn(async {
        let addr = "0.0.0.0:6181";
        let listener = TcpListener::bind(&addr).await.unwrap();
        println!("Listening on: {}", addr);
        let (mut socket, _) = listener.accept().await.unwrap();
        println!("Connection from {:#?}", socket);
        let mut buf = vec![0; 1024];

        let n = socket
            .read(&mut buf)
            .await
            .expect("failed to read data from socket");

        if n == 0 {
            return;
        }

        socket
            .write_all(&buf[0..n])
            .await
            .expect("failed to write data to socket");
        println!("cya cowboy!...");
    });

    _ = tokio::join!(h1, h2, h3, h4);
}
