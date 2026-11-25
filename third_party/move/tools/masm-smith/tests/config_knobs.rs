// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that generated tests stay within each size limit of the config, across many seeds.

use arbitrary::Unstructured;
use masm_smith::{generate, Config, Step};

const ITERATIONS: u32 = 64;
const SEED_BYTES: usize = 128;

fn seed_for(i: u32) -> Vec<u8> {
    (0u32..SEED_BYTES as u32)
        .map(|b| ((b.wrapping_mul(31).wrapping_add(i.wrapping_mul(17))) ^ 0xA5) as u8)
        .collect()
}

#[test]
fn module_count_respects_max() {
    let cfg = Config {
        module_count_max: 1,
        ..Config::default()
    };
    let mut max_seen = 0;
    for i in 0..ITERATIONS {
        let seed = seed_for(i);
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, &cfg) else {
            continue;
        };
        let publish_count = test
            .steps
            .iter()
            .filter(|s| matches!(s, Step::Publish(_)))
            .count();
        assert!(
            publish_count <= 1,
            "iter #{i}: {} publishes exceeds module_count_max=1",
            publish_count
        );
        max_seen = max_seen.max(publish_count);
    }
    assert!(
        max_seen > 0,
        "no module was ever published in {ITERATIONS} iters"
    );
}

#[test]
fn structs_per_module_respects_max() {
    let cfg = Config {
        structs_per_module_max: 2,
        ..Config::default()
    };
    for i in 0..ITERATIONS {
        let seed = seed_for(i);
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, &cfg) else {
            continue;
        };
        for step in &test.steps {
            if let Step::Publish(module) = step {
                assert!(
                    module.structs.len() <= 2,
                    "iter #{i}: module {} has {} structs > max 2",
                    module.id.name(),
                    module.structs.len()
                );
            }
        }
    }
}

#[test]
fn functions_per_module_respects_max() {
    let cfg = Config {
        functions_per_module_max: 2,
        ..Config::default()
    };
    for i in 0..ITERATIONS {
        let seed = seed_for(i);
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, &cfg) else {
            continue;
        };
        for step in &test.steps {
            if let Step::Publish(module) = step {
                // Struct API wrappers (`$`-delimited names) are exempt from the cap.
                let generated = module
                    .functions
                    .iter()
                    .filter(|f| !f.name.as_str().contains('$'))
                    .count();
                assert!(
                    generated <= 2,
                    "iter #{i}: module {} has {} functions > max 2",
                    module.id.name(),
                    generated
                );
            }
        }
    }
}

#[test]
fn locals_per_function_respects_max() {
    let cfg = Config {
        locals_per_fun_max: 2,
        ..Config::default()
    };
    for i in 0..ITERATIONS {
        let seed = seed_for(i);
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, &cfg) else {
            continue;
        };
        for step in &test.steps {
            if let Step::Publish(module) = step {
                for f in &module.functions {
                    assert!(
                        f.locals.len() <= 2,
                        "iter #{i}: fn {} has {} locals > max 2",
                        f.name,
                        f.locals.len()
                    );
                }
            }
        }
    }
}

#[test]
fn zero_structs_means_no_structs() {
    let cfg = Config {
        structs_per_module_max: 0,
        ..Config::default()
    };
    for i in 0..ITERATIONS {
        let seed = seed_for(i);
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, &cfg) else {
            continue;
        };
        for step in &test.steps {
            if let Step::Publish(module) = step {
                assert!(
                    module.structs.is_empty(),
                    "iter #{i}: module {} has structs despite max=0",
                    module.id.name()
                );
            }
        }
    }
}

#[test]
fn type_name_get_respects_knob() {
    let calls_with = |enable_type_name_get: bool| {
        let cfg = Config {
            enable_type_name_get,
            ..Config::default()
        };
        (0..ITERATIONS)
            .filter_map(|i| generate(&mut Unstructured::new(&seed_for(i)), &cfg).ok())
            .filter(|test| test.calls_type_name_get())
            .count()
    };
    assert!(
        calls_with(true) > 0,
        "no test called type_name::get in {ITERATIONS} iters"
    );
    assert_eq!(calls_with(false), 0, "type_name::get called while disabled");
}
