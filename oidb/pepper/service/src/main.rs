// Copyright © Aptos Foundation

use aptos_oidb_pepper_common::{PepperRequest};
use aptos_oidb_pepper_service::{
    about::ABOUT_JSON,
    jwk,
    vuf_keys::{VUF_SCHEME0_SK, VUF_VERIFICATION_KEY_JSON},
};
use hyper::{
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
    },
    service::{make_service_fn, service_fn},
    Body, Method, Server, StatusCode,
};
use log::LevelFilter;
use std::{convert::Infallible, net::SocketAddr, ops::Deref, time::Duration};

async fn handle_request(req: hyper::Request<Body>) -> Result<hyper::Response<Body>, Infallible> {
    let origin = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/about") => hyper::Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(ABOUT_JSON.as_str()))
            .expect("Response should build"),
        (&Method::GET, "/vuf-pub-key") => hyper::Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(VUF_VERIFICATION_KEY_JSON.as_str()))
            .expect("Response should build"),
        (&Method::POST, "/") => {
            let body = req.into_body();
            let body_bytes = hyper::body::to_bytes(body).await.unwrap_or_default();
            let request = serde_json::from_slice::<PepperRequest>(&body_bytes).unwrap();
            let response = aptos_oidb_pepper_service::process(request).await;
            let json = serde_json::to_string_pretty(&response).unwrap();
            hyper::Response::builder()
                .status(StatusCode::OK)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
                .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
                .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
                .header(ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .expect("Response should build")
        },
        (&Method::OPTIONS, _) => hyper::Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization")
            .body(Body::empty())
            .expect("Response should build"),
        _ => {
            let mut response = hyper::Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_FOUND;
            response
        },
    };
    Ok(response)
}

#[tokio::main]
async fn main() {
    // Trigger private key loading.
    let _ = VUF_SCHEME0_SK.deref();

    env_logger::Builder::new()
        .filter(None, LevelFilter::Info)
        .init();

    // TODO: JWKs should be from on-chain states?
    jwk::start_jwk_refresh_loop(
        "https://accounts.google.com",
        "https://www.googleapis.com/oauth2/v3/certs",
        Duration::from_secs(10),
    );
    jwk::start_jwk_refresh_loop(
        "https://www.facebook.com",
        "https://www.facebook.com/.well-known/oauth/openid/jwks",
        Duration::from_secs(10),
    );
    jwk::start_jwk_refresh_loop(
        "https://id.twitch.tv/oauth2",
        "https://id.twitch.tv/oauth2/keys",
        Duration::from_secs(10),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
