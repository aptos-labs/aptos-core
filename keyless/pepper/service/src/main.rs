// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_common::{BadPepperRequestError, PepperRequest};
use aptos_keyless_pepper_service::{
    about::ABOUT_JSON,
    jwk,
    metrics::{start_metric_server, REQUEST_HANDLING_SECONDS},
    process,
    vuf_keys::{PEPPER_V0_VUF_VERIFICATION_KEY_JSON, VUF_SK},
    ProcessingFailure::{BadRequest, InternalError},
};
use aptos_logger::info;
use hyper::{
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
    },
    service::{make_service_fn, service_fn},
    Body, Method, Server, StatusCode,
};
use std::{
    convert::Infallible,
    net::SocketAddr,
    ops::Deref,
    time::{Duration, Instant},
};
use uuid::Uuid;

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
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(ABOUT_JSON.as_str()))
            .expect("Response should build"),
        (&Method::GET, "/v0/vuf-pub-key") | (&Method::GET, "/v1/vuf-pub-key") => hyper::Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(PEPPER_V0_VUF_VERIFICATION_KEY_JSON.as_str()))
            .expect("Response should build"),
        (&Method::POST, "/v0/fetch") => {
            let session_id = Uuid::new_v4();
            info!(session_id = session_id, "New pepper request v0!");
            let timer = Instant::now();
            let body = req.into_body();
            let body_bytes = hyper::body::to_bytes(body).await.unwrap_or_default();
            let pepper_request = serde_json::from_slice::<PepperRequest>(&body_bytes);
            let pepper_response = pepper_request.map(|req| process(&session_id, req));
            let processing_time = timer.elapsed();
            info!(
                session_id = session_id,
                microseconds_processing = processing_time.as_micros(),
                "PepperResponse generated: {:?}.",
                pepper_response
            );
            let result_str = pepper_response.is_ok().to_string();
            REQUEST_HANDLING_SECONDS
                .with_label_values(&["v0", result_str.as_str()])
                .observe(processing_time.as_secs_f64());
            let (status_code, body_json) = match pepper_response {
                Ok(Ok(pepper_response)) => (
                    StatusCode::OK,
                    serde_json::to_string_pretty(&pepper_response).unwrap(),
                ),
                Ok(Err(BadRequest(err))) => (
                    StatusCode::BAD_REQUEST,
                    serde_json::to_string_pretty(&BadPepperRequestError {
                        message: err.to_string(),
                    })
                    .unwrap(),
                ),
                Ok(Err(InternalError(_))) => (StatusCode::INTERNAL_SERVER_ERROR, String::new()),
                Err(err) => (
                    StatusCode::BAD_REQUEST,
                    serde_json::to_string_pretty(&BadPepperRequestError {
                        message: err.to_string(),
                    })
                    .unwrap(),
                ),
            };
            hyper::Response::builder()
                .status(status_code)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
                .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
                .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
                .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body_json))
                .expect("Response should build")
        },
        (&Method::POST, "/v1/fetch") => {
            let body = req.into_body();
            let body_bytes = hyper::body::to_bytes(body).await.unwrap_or_default();
            let pepper_request = serde_json::from_slice::<PepperRequestV1>(&body_bytes);
            let pepper_response = pepper_request.map(v1::process);
            let (status_code, body_json) = match pepper_response {
                Ok(Ok(pepper_response)) => (
                    StatusCode::OK,
                    serde_json::to_string_pretty(&pepper_response).unwrap(),
                ),
                Ok(Err(v1::ProcessingFailure::BadRequest(err))) => (
                    StatusCode::BAD_REQUEST,
                    serde_json::to_string_pretty(&BadPepperRequestError {
                        message: err.to_string(),
                    })
                        .unwrap(),
                ),
                Ok(Err(v1::ProcessingFailure::InternalError(_))) => (StatusCode::INTERNAL_SERVER_ERROR, String::new()),
                Err(err) => (
                    StatusCode::BAD_REQUEST,
                    serde_json::to_string_pretty(&BadPepperRequestError {
                        message: err.to_string(),
                    })
                        .unwrap(),
                ),
            };
            let response = hyper::Response::builder()
                .status(status_code)
                .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
                .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
                .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
                .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body_json))
                .expect("Response should build");
            info!(session_id = session_id, "HTTP response built.");
            response
        },
        (&Method::OPTIONS, _) => hyper::Response::builder()
            .status(StatusCode::OK)
            .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
            .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
            .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
            .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
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
    let _ = VUF_SK.deref();

    aptos_logger::Logger::new().init();
    start_metric_server();

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
