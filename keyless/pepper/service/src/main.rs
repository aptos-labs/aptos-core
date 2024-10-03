// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_pepper_common::BadPepperRequestError;
use aptos_keyless_pepper_service::{
    about::ABOUT_JSON,
    account_db::{init_account_db, ACCOUNT_RECOVERY_DB},
    account_managers::ACCOUNT_MANAGERS,
    jwk::{self, parse_jwks, DECODING_KEY_CACHE},
    metrics::start_metric_server,
    vuf_keys::{PEPPER_VUF_VERIFICATION_KEY_JSON, VUF_SK},
    HandlerTrait,
    ProcessingFailure::{BadRequest, InternalError},
    V0FetchHandler, V0SignatureHandler, V0VerifyHandler,
};
use aptos_logger::{error, info};
use aptos_types::keyless::test_utils::get_sample_iss;
use hyper::{
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
    },
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{convert::Infallible, fmt::Debug, net::SocketAddr, ops::Deref, time::Duration};

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let origin = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/about") => {
            build_response(origin, StatusCode::OK, ABOUT_JSON.deref().clone())
        },
        (&Method::GET, "/v0/vuf-pub-key") => build_response(
            origin,
            StatusCode::OK,
            PEPPER_VUF_VERIFICATION_KEY_JSON.deref().clone(),
        ),
        (&Method::POST, "/v0/signature") => {
            generate_response(origin, req, &V0SignatureHandler).await
        },
        (&Method::POST, "/v0/verify") => generate_response(origin, req, &V0VerifyHandler).await,
        (&Method::POST, "/v0/fetch") => generate_response(origin, req, &V0FetchHandler).await,
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
    let _ = ACCOUNT_MANAGERS.deref();
    {
        let _db = ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await;
    }
    aptos_logger::Logger::new().init();
    start_metric_server();

    // TODO: JWKs should be from on-chain states?
    jwk::start_jwk_refresh_loop(
        "https://accounts.google.com",
        "https://www.googleapis.com/oauth2/v3/certs",
        Duration::from_secs(10),
    );
    jwk::start_jwk_refresh_loop(
        "https://appleid.apple.com",
        "https://appleid.apple.com/auth/keys",
        Duration::from_secs(10),
    );

    let test_jwk = include_str!("../../../../types/src/jwks/rsa/secure_test_jwk.json");
    DECODING_KEY_CACHE.insert(
        get_sample_iss(),
        parse_jwks(test_jwk).expect("test jwk should parse"),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

/// Feed a request into a handler and wrap the output as an HTTP response.
async fn generate_response<PREQ, PRES, HDLR>(
    origin: String,
    req: Request<Body>,
    handler: &HDLR,
) -> Response<Body>
where
    PREQ: Debug + Serialize + DeserializeOwned,
    PRES: Debug + Serialize,
    HDLR: HandlerTrait<PREQ, PRES> + Send + Sync,
{
    let body = req.into_body();
    let body_bytes = hyper::body::to_bytes(body).await.unwrap_or_default();
    let pepper_request = serde_json::from_slice::<PREQ>(&body_bytes);
    let (status_code, body_json) = match pepper_request {
        Ok(request) => {
            let pepper_response = handler.handle(request).await;
            match pepper_response {
                Ok(pepper_response) => {
                    info!("Request processed normally.");
                    (
                        StatusCode::OK,
                        serde_json::to_string_pretty(&pepper_response).unwrap(),
                    )
                },
                Err(BadRequest(err)) => {
                    info!("Processing failed with bad request: {err}");
                    (
                        StatusCode::BAD_REQUEST,
                        serde_json::to_string_pretty(&BadPepperRequestError {
                            message: err.to_string(),
                        })
                        .unwrap(),
                    )
                },
                Err(InternalError(e)) => {
                    error!("Processing failed with internal error: {e}");
                    (StatusCode::INTERNAL_SERVER_ERROR, String::new())
                },
            }
        },
        Err(err) => {
            error!("Processing did not start due to bad request: {err}");
            (
                StatusCode::BAD_REQUEST,
                serde_json::to_string_pretty(&BadPepperRequestError {
                    message: err.to_string(),
                })
                .unwrap(),
            )
        },
    };

    build_response(origin, status_code, body_json)
}

fn build_response(origin: String, status_code: StatusCode, body_str: String) -> Response<Body> {
    hyper::Response::builder()
        .status(status_code)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
        .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
        .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
        .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body_str))
        .expect("Response should build")
}
