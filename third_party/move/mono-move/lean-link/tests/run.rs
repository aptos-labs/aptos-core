// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end adapter tests: a compiled scalar fixture runs through the
//! plain Rust entry the C ABI wraps.

use mono_move_lean_link::{payload as p, run_bytes};

const SOURCE: &str = r#"
module 0x42::scalar {
    use std::vector;

    public fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun abort_if_large(x: u64): u64 {
        assert!(x <= 100, 42);
        x
    }

    public fun burn(): u64 {
        let i = 0;
        while (i < 1000000000) {
            i = i + 1;
        };
        i
    }

    public fun sum(values: vector<u64>): u64 {
        let total = 0;
        vector::for_each_ref(&values, |value| {
            total = total + *value;
        });
        total
    }

    public fun make_pair(a: u64, b: u64): vector<u64> {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        v
    }

    public fun echo_address(a: address): address {
        a
    }

    public fun wide(x: u256): u256 {
        x + 1
    }

    public fun overflow(x: u8): u8 {
        x + 200
    }

    public fun out_of_bounds(index: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, 7);
        *vector::borrow(&v, index)
    }

    public fun grow(): vector<u64> {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < 100000) {
            vector::push_back(&mut v, i);
            i = i + 1;
        };
        v
    }
}
"#;

fn request(calls: Vec<p::Call>, gas: u64) -> Vec<u8> {
    request_with_limits(calls, p::Limits { gas, heap: None })
}

fn request_with_limits(calls: Vec<p::Call>, limits: p::Limits) -> Vec<u8> {
    let request = p::Request {
        version: p::PAYLOAD_VERSION,
        compile: p::CompileSpec {
            sources: vec![p::SourceFile {
                name: "scalar.move".to_string(),
                text: SOURCE.to_string(),
            }],
            addresses: Default::default(),
            language: 2,
        },
        limits,
        calls,
    };
    serde_json::to_vec(&request).expect("request serialization is infallible")
}

fn call(function: &str, args: Vec<p::Value>) -> p::Call {
    p::Call {
        function: function.to_string(),
        signers: vec![],
        args,
    }
}

fn integer(width: u16, signed: bool, value: &str) -> p::Value {
    p::Value::Integer {
        width,
        signed,
        value: value.to_string(),
    }
}

fn vector_of(elements: Vec<p::Value>) -> p::Value {
    p::Value::Vector { elements }
}

fn address(value: &str) -> p::Value {
    p::Value::Address {
        value: value.to_string(),
    }
}

fn run(bytes: &[u8]) -> p::Response {
    let response = mono_move_lean_link::run_bytes(bytes);
    serde_json::from_slice(&response).expect("the response decodes")
}

#[test]
fn a_scalar_call_and_an_abort_return_normalized_outcomes() {
    let response = run(&request(
        vec![
            call("0x42::scalar::add", vec![
                integer(64, false, "1"),
                integer(64, false, "2"),
            ]),
            call("0x42::scalar::abort_if_large", vec![integer(
                64, false, "200",
            )]),
        ],
        10_000_000_000,
    ));
    assert_eq!(response.version, p::PAYLOAD_VERSION);
    assert_eq!(response.identity.abi, mono_move_lean_link::ABI_VERSION);
    assert_eq!(response.outcomes.len(), 2);

    let p::Outcome::Returned {
        values,
        gas_used,
        gc_count: _,
    } = &response.outcomes[0]
    else {
        panic!(
            "expected the addition to return, got {:?}",
            response.outcomes[0]
        );
    };
    assert_eq!(values, &vec![integer(64, false, "3")]);
    assert!(*gas_used > 0);

    let p::Outcome::Aborted {
        code,
        location: _,
        message: _,
    } = &response.outcomes[1]
    else {
        panic!(
            "expected the guard to abort, got {:?}",
            response.outcomes[1]
        );
    };
    assert_eq!(*code, 42);
}

#[test]
fn a_runtime_failure_is_propagated_as_a_program_outcome() {
    // MonoVM reports arithmetic overflow and an out-of-bounds index as typed
    // execution errors rather than aborts. They are outcomes of the program,
    // so they must reach the caller as `failed` with the kind to branch on —
    // not as an adapter `error`, which claims no outcome was obtained.
    let response = run(&request(
        vec![
            call("0x42::scalar::overflow", vec![integer(8, false, "100")]),
            call("0x42::scalar::out_of_bounds", vec![integer(64, false, "5")]),
        ],
        10_000_000_000,
    ));
    assert_eq!(response.outcomes.len(), 2);
    for outcome in &response.outcomes {
        let p::Outcome::Failed { failure, message } = outcome else {
            panic!("expected a propagated runtime failure, got {outcome:?}");
        };
        assert_eq!(failure, "InvalidOperation");
        assert!(!message.is_empty(), "the diagnostic message is carried");
    }
}

#[test]
fn identical_requests_produce_identical_responses() {
    let request = request(
        vec![call("0x42::scalar::add", vec![
            integer(64, false, "20"),
            integer(64, false, "22"),
        ])],
        10_000_000_000,
    );
    assert_eq!(run_bytes(&request), run_bytes(&request));
}

#[test]
fn gas_exhaustion_is_reported_as_exhausted() {
    let response = run(&request(vec![call("0x42::scalar::burn", vec![])], 1_000));
    assert_eq!(response.outcomes, vec![p::Outcome::Exhausted {
        resource: p::ExhaustedResource::Gas,
    }]);
}

#[test]
fn an_unknown_function_is_an_error_outcome() {
    let response = run(&request(
        vec![call("0x42::scalar::missing", vec![])],
        10_000_000_000,
    ));
    assert!(matches!(response.outcomes.as_slice(), [
        p::Outcome::Error {
            stage: p::Stage::Run,
            ..
        }
    ]));
}

#[test]
fn a_malformed_function_reference_is_an_abi_error_outcome() {
    let response = run(&request(vec![call("scalar::add", vec![])], 10_000_000_000));
    assert!(matches!(response.outcomes.as_slice(), [
        p::Outcome::Error {
            stage: p::Stage::Abi,
            ..
        }
    ]));
}

#[test]
fn a_broken_source_is_a_compile_error_outcome_for_every_call() {
    let mut request = p::Request {
        version: p::PAYLOAD_VERSION,
        compile: p::CompileSpec {
            sources: vec![p::SourceFile {
                name: "broken.move".to_string(),
                text: "module 0x42::broken { public fun f(): u64 { } }".to_string(),
            }],
            addresses: Default::default(),
            language: 2,
        },
        limits: p::Limits {
            gas: 10_000_000_000,
            heap: None,
        },
        calls: vec![
            call("0x42::broken::f", vec![]),
            call("0x42::broken::f", vec![]),
        ],
    };
    request.version = p::PAYLOAD_VERSION;
    let bytes = serde_json::to_vec(&request).expect("request serialization is infallible");
    let response = run(&bytes);
    assert_eq!(response.outcomes.len(), 2);
    assert!(response
        .outcomes
        .iter()
        .all(|outcome| matches!(outcome, p::Outcome::Error {
            stage: p::Stage::Compile,
            ..
        })));
}

#[test]
fn vector_arguments_and_results_round_trip() {
    let response = run(&request(
        vec![
            call("0x42::scalar::sum", vec![vector_of(vec![
                integer(64, false, "10"),
                integer(64, false, "20"),
                integer(64, false, "12"),
            ])]),
            call("0x42::scalar::make_pair", vec![
                integer(64, false, "7"),
                integer(64, false, "9"),
            ]),
        ],
        10_000_000_000,
    ));
    let [first, second] = &response.outcomes[..] else {
        panic!("expected two outcomes, got {:?}", response.outcomes)
    };
    let p::Outcome::Returned {
        values, gas_used, ..
    } = first
    else {
        panic!("expected the sum to return, got {first:?}")
    };
    assert_eq!(values, &vec![integer(64, false, "42")]);
    assert!(*gas_used > 0);
    let p::Outcome::Returned {
        values, gas_used, ..
    } = second
    else {
        panic!("expected the pair to return, got {second:?}")
    };
    assert_eq!(values, &vec![vector_of(vec![
        integer(64, false, "7"),
        integer(64, false, "9"),
    ])]);
    assert!(*gas_used > 0);
}

#[test]
fn address_and_wide_integer_results_round_trip() {
    let response = run(&request(
        vec![
            call("0x42::scalar::echo_address", vec![address("0x1234")]),
            call("0x42::scalar::wide", vec![integer(
                256,
                false,
                "340282366920938463463374607431768211455",
            )]),
        ],
        10_000_000_000,
    ));
    let [first, second] = &response.outcomes[..] else {
        panic!("expected two outcomes, got {:?}", response.outcomes)
    };
    let p::Outcome::Returned { values, .. } = first else {
        panic!("expected the address echo to return, got {first:?}")
    };
    // The result renders in the canonical short hexadecimal form.
    assert_eq!(values, &vec![address("0x1234")]);
    let p::Outcome::Returned { values, .. } = second else {
        panic!("expected the wide addition to return, got {second:?}")
    };
    assert_eq!(values, &vec![integer(
        256,
        false,
        "340282366920938463463374607431768211456"
    )]);
}

#[test]
fn heap_exhaustion_is_reported_as_exhausted() {
    let response = run(&request_with_limits(
        vec![call("0x42::scalar::grow", vec![])],
        p::Limits {
            gas: 10_000_000_000,
            heap: Some(1024),
        },
    ));
    assert_eq!(response.outcomes, vec![p::Outcome::Exhausted {
        resource: p::ExhaustedResource::Heap,
    }]);
}
