// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that mutation and splicing keep tests verifiable and runnable, and that the corpus
//! encoding round-trips.

mod common;

use arbitrary::Unstructured;
use common::{assert_verifies, random_bytes, run_output, valid_program};
use masm_smith::{
    generate,
    ir::{Operand, Step, StructLayout, Type},
    mutate::{decode, encode, mutate, MUTATED_STEPS_MAX},
    splice::splice,
    Config, MasmTest,
};
use std::collections::{BTreeMap, BTreeSet};

const PROGRAMS: u32 = 96;
const ROUNDS: u32 = 16;

fn bytes_for(seed: u32, len: u32) -> Vec<u8> {
    (0..len)
        .map(|offset| (seed.wrapping_mul(2654435761).wrapping_add(offset * 40503) >> 7) as u8)
        .collect()
}

#[test]
fn mutations_keep_tests_verifiable() {
    let mut applied = 0u32;
    for program in 0..PROGRAMS {
        let seed = bytes_for(program, 256);
        let Ok(mut test) = generate(&mut Unstructured::new(&seed), &Config::default()) else {
            continue;
        };
        for round in 0..ROUNDS {
            let mutation_bytes = bytes_for(program * 1000 + round, 512);
            if mutate(
                &mut test,
                &Config::default(),
                &mut Unstructured::new(&mutation_bytes),
            )
            .unwrap_or(false)
            {
                applied += 1;
                assert_verifies(&test, &format!("program {program} round {round}"));
            }
        }
    }
    eprintln!("applied {applied} mutations");
    assert!(applied > PROGRAMS, "only {applied} mutations applied");
}

#[test]
fn encoding_round_trips() {
    for program in 0..PROGRAMS {
        let seed = bytes_for(program, 256);
        let Ok(test) = generate(&mut Unstructured::new(&seed), &Config::default()) else {
            continue;
        };
        let decoded = decode(&encode(&test)).expect("encoded test decodes");
        assert_eq!(decoded.render(), test.render(), "program {program}");
    }
}

#[test]
fn decode_rejects_other_versions() {
    let test = generate(
        &mut Unstructured::new(&bytes_for(0, 256)),
        &Config::default(),
    )
    .expect("generate");
    let mut bytes = encode(&test);
    bytes[0] = bytes[0].wrapping_add(1);
    assert!(decode(&bytes).is_none());
}

#[test]
fn decode_rejects_corrupted_bytes() {
    let test = generate(
        &mut Unstructured::new(&bytes_for(0, 256)),
        &Config::default(),
    )
    .expect("generate");
    let bytes = encode(&test);
    for position in [1, bytes.len() / 2, bytes.len() - 1] {
        let mut corrupted = bytes.clone();
        corrupted[position] ^= 0x01;
        assert!(decode(&corrupted).is_none(), "byte {position} flip decoded");
    }
}

/// The control sequence of every generated loop: read the counter, compare with zero, exit,
/// decrement, store. The header is the block whose first instruction reads the counter.
fn loop_headers(test: &MasmTest) -> Vec<String> {
    test.render()
        .lines()
        .collect::<Vec<_>>()
        .windows(8)
        .filter(|window| {
            window[0].contains(": copy_loc __loop_counter")
                && window
                    .last()
                    .is_some_and(|line| line.ends_with("st_loc __loop_counter"))
        })
        .map(|window| window.join("\n"))
        .collect()
}

#[test]
fn mutations_preserve_loop_bounds() {
    let mut checked = 0u32;
    let mut checked_headers = 0u32;
    for program in 0..PROGRAMS {
        let seed = bytes_for(program, 256);
        let Ok(original) = generate(&mut Unstructured::new(&seed), &Config::default()) else {
            continue;
        };
        let headers = loop_headers(&original);
        if headers.is_empty() {
            continue;
        }
        let mut test = original.clone();
        for round in 0..64u32 {
            let mutation_bytes = bytes_for(program * 1000 + round, 512);
            let _ = mutate(
                &mut test,
                &Config::default(),
                &mut Unstructured::new(&mutation_bytes),
            );
        }
        // Regenerating a body can drop or add loops, so compare only bodies still present.
        for header in loop_headers(&test) {
            assert!(
                header.contains("ld_u64 0\n  gt\n") && header.contains("ld_u64 1\n  sub\n"),
                "program {program}: loop control changed\n{header}"
            );
            checked_headers += 1;
        }
        checked += 1;
    }
    assert!(
        checked > 0 && checked_headers > 0,
        "no loop headers checked ({checked} programs)"
    );
}

/// The generator config the VM fuzz targets use: friends are unsupported by the runner, and
/// global storage operations are favored.
#[test]
fn splices_keep_tests_verifiable() {
    let programs: Vec<MasmTest> = (0..PROGRAMS).filter_map(valid_program).collect();
    let mut spliced = 0u32;
    for (index, recipient) in programs.iter().enumerate() {
        let donor = &programs[(index * 7 + 3) % programs.len()];
        let choices = bytes_for(index as u32 + 5000, 64);
        let Some(test) = splice(recipient, donor, &mut Unstructured::new(&choices))
            .expect("splice draws from enough bytes")
        else {
            continue;
        };
        assert!(test.steps.len() <= MUTATED_STEPS_MAX);
        assert_verifies(&test, &format!("splice {index}"));
        spliced += 1;
    }
    assert!(spliced > PROGRAMS / 4, "only {spliced} splices applied");
}

/// Counts each mutation outcome by rendered task shape: upgrades, extra runs.
fn publish_counts(test: &MasmTest) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for step in &test.steps {
        if let Step::Publish(module) = step {
            *counts.entry(module.id.name().to_string()).or_insert(0) += 1;
        }
    }
    counts
}

/// Mutated and spliced programs publish and run in the VM: no runner error, no module that
/// fails to publish, and at least some upgrades that take effect.
#[test]
fn mutated_and_spliced_programs_run() {
    let programs: Vec<MasmTest> = (0..PROGRAMS).filter_map(valid_program).collect();
    let mut upgrades = 0u32;
    let mut cross_transaction_states = 0u32;
    for (index, original) in programs.iter().enumerate() {
        let donor = &programs[(index * 5 + 1) % programs.len()];
        let mut test = splice(
            original,
            donor,
            &mut Unstructured::new(&random_bytes(index as u32 + 9000, 64)),
        )
        .expect("splice draws from enough bytes")
        .unwrap_or_else(|| original.clone());
        for round in 0..ROUNDS {
            let mutation_bytes = random_bytes(index as u32 * 1000 + round + 77, 1024);
            let _ = mutate(
                &mut test,
                &Config::vm_fuzzing(),
                &mut Unstructured::new(&mutation_bytes),
            );
        }
        upgrades += publish_counts(&test)
            .values()
            .filter(|count| **count > 1)
            .count() as u32;
        let output = run_output(&test, &format!("program {index}"));
        assert!(
            !output.contains("Unable to publish module"),
            "program {index}: a module failed to publish\n{output}\n{}",
            test.render()
        );
        cross_transaction_states += output.matches("RESOURCE_ALREADY_EXISTS").count() as u32;
    }
    assert!(upgrades > 0, "no module upgrade was exercised");
    assert!(
        cross_transaction_states > 0,
        "no transaction observed a resource stored by an earlier one"
    );
}

#[test]
fn vm_config_programs_have_no_call_cycles() {
    let programs: Vec<MasmTest> = (0..PROGRAMS).filter_map(valid_program).collect();
    for (index, original) in programs.iter().enumerate() {
        assert!(
            !has_call_cycle(original),
            "program {index}: generated call cycle"
        );
        let donor = &programs[(index * 7 + 3) % programs.len()];
        let mut test = splice(
            original,
            donor,
            &mut Unstructured::new(&random_bytes(index as u32 + 7000, 64)),
        )
        .expect("splice draws from enough bytes")
        .unwrap_or_else(|| original.clone());
        for round in 0..ROUNDS * 2 {
            let mutation_bytes = random_bytes(index as u32 * 1000 + round + 3, 16 * 1024);
            let _ = mutate(
                &mut test,
                &Config::vm_fuzzing(),
                &mut Unstructured::new(&mutation_bytes),
            );
        }
        assert!(
            !has_call_cycle(&test),
            "program {index}: call cycle after mutation\n{}",
            test.render()
        );
    }
}

/// Bounded recursion verifies, has no other call cycles, and includes successful recursive runs.
#[test]
fn bounded_recursion_verifies_and_terminates() {
    let cfg = Config {
        bounded_recursion_prob: 255,
        ..Config::vm_fuzzing()
    };
    let (mut recursive_programs, mut returned) = (0u32, 0u32);
    for seed in 0..PROGRAMS {
        let Ok(test) = generate(&mut Unstructured::new(&random_bytes(seed, 16 * 1024)), &cfg)
        else {
            continue;
        };
        if !test.injected_violations.is_empty() || test.post_process.is_some() {
            continue;
        }
        let text = test.render();
        if !text.contains("  neq\n  br_false") {
            continue;
        }
        recursive_programs += 1;
        assert_verifies(&test, &format!("recursive program {seed}"));
        assert!(
            !has_call_cycle(&test),
            "program {seed}: unbounded recursion"
        );
        let output = run_output(&test, &format!("program {seed}"));
        returned += recursive_calls_that_returned(&text, &output);
    }
    assert!(recursive_programs > 0, "no function was made recursive");
    assert!(returned > 0, "no recursive call returned");
}

/// Run steps that call a recursive function directly and finish without an error. A recursive
/// function's body starts with the depth check `copy_loc; ld_u64 0; neq; br_false`.
fn recursive_calls_that_returned(text: &str, output: &str) -> u32 {
    let lines: Vec<&str> = text.lines().collect();
    let recursive: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" fun ") && !line.starts_with("//"))
        .filter(|(index, _)| {
            let prologue: Vec<&str> = lines[index + 1..]
                .iter()
                .copied()
                .skip_while(|line| line.trim_start().starts_with("local "))
                .take(4)
                .collect();
            prologue.len() == 4
                && prologue[0].starts_with("  copy_loc ")
                && prologue[1] == "  ld_u64 0"
                && prologue[2] == "  neq"
                && prologue[3].starts_with("  br_false")
        })
        .filter_map(|(_, line)| {
            let (head, _) = line.split_once('(')?;
            head.rsplit(' ').next()?.split('<').next()
        })
        .collect();
    output
        .split("\ntask ")
        .skip(1)
        .filter(|task| {
            let header = task.lines().next().unwrap_or_default();
            let Some((_, call)) = header.split_once(":  run 0x") else {
                return false;
            };
            let target = call.split_whitespace().next().unwrap_or_default();
            recursive
                .iter()
                .any(|name| target.ends_with(&format!("::{name}")))
                && !task.contains("Error:")
        })
        .count() as u32
}

/// Whether a function body starts with the bounded-recursion depth check.
fn has_recursion_prologue(function: &masm_smith::ir::Function) -> bool {
    let Some(entry) = function.body.first() else {
        return false;
    };
    matches!(
        entry
            .instrs
            .iter()
            .map(|instr| instr.opcode.as_str())
            .collect::<Vec<_>>()
            .as_slice(),
        ["copy_loc", "ld_u64", "neq", "br_false"]
    )
}

/// Detects same-module cycles through calls or packed closures, excluding self-edges of
/// functions with a bounded-recursion prologue.
fn has_call_cycle(test: &MasmTest) -> bool {
    test.steps.iter().any(|step| {
        let Step::Publish(module) = step else {
            return false;
        };
        let callees: Vec<Vec<usize>> = module
            .functions
            .iter()
            .enumerate()
            .map(|(caller, function)| {
                function
                    .body
                    .iter()
                    .flat_map(|block| &block.instrs)
                    .filter_map(|instr| match instr.operands.first() {
                        Some(Operand::Function(target, _))
                            if matches!(instr.opcode.as_str(), "call" | "pack_closure")
                                && target.module == module.id =>
                        {
                            module
                                .functions
                                .iter()
                                .position(|candidate| candidate.name == target.name)
                        },
                        _ => None,
                    })
                    .filter(|callee| *callee != caller || !has_recursion_prologue(function))
                    .collect()
            })
            .collect();
        (0..callees.len()).any(|start| {
            let mut pending = callees[start].clone();
            let mut visited = vec![false; callees.len()];
            while let Some(node) = pending.pop() {
                if node == start {
                    return true;
                }
                if !std::mem::replace(&mut visited[node], true) {
                    pending.extend(&callees[node]);
                }
            }
            false
        })
    })
}

/// Closure recursion re-enters its module and exercises the VM's resource lock.
#[test]
fn closure_recursion_reaches_reentrancy_lock() {
    unsafe { std::env::set_var("MOVE_TEST_DEBUG", "1") };
    let cfg = Config {
        bounded_recursion_prob: 255,
        ..Config::vm_fuzzing()
    };
    let mut locked = 0u32;
    for seed in 0..PROGRAMS * 4 {
        let Ok(test) = generate(&mut Unstructured::new(&random_bytes(seed, 16 * 1024)), &cfg)
        else {
            continue;
        };
        if !test.injected_violations.is_empty() || test.post_process.is_some() {
            continue;
        }
        let text = test.render();
        if !text.contains("call_closure") {
            continue;
        }
        let output = run_output(&test, &format!("program {seed}"));
        locked += output.matches("because of active reentrancy").count() as u32;
    }
    assert!(locked > 0, "no resource access hit the reentrancy lock");
}

/// Deep bounded recursion reaches both stack limits: values held across each recursive call
/// fill the operand stack shared by all frames, and the frames fill the call stack.
#[test]
fn deep_bounded_recursion_reaches_stack_limits() {
    let cfg = Config {
        bounded_recursion_prob: 255,
        ..Config::vm_fuzzing()
    };
    let (mut operand_overflows, mut call_overflows) = (0u32, 0u32);
    for seed in 0..PROGRAMS * 8 {
        if operand_overflows > 0 && call_overflows > 0 {
            break;
        }
        let Ok(test) = generate(&mut Unstructured::new(&random_bytes(seed, 16 * 1024)), &cfg)
        else {
            continue;
        };
        if !test.injected_violations.is_empty() || test.post_process.is_some() {
            continue;
        }
        let text = test.render();
        if !text.contains("  neq\n  br_false") {
            continue;
        }
        let output = run_output(&test, &format!("program {seed}"));
        operand_overflows += output.matches("EXECUTION_STACK_OVERFLOW").count() as u32;
        call_overflows += output.matches("CALL_STACK_OVERFLOW").count() as u32;
    }
    assert!(
        operand_overflows > 0,
        "no recursion reached the operand-stack limit"
    );
    assert!(
        call_overflows > 0,
        "no recursion reached the call-stack limit"
    );
}

/// Values worked on in place verify, run, and reach the VM's operations on values behind
/// references: vector mutation, element and variant-field writes, and stdlib vector calls.
#[test]
fn in_place_programs_verify_and_run() {
    let cfg = Config {
        in_place_prob: 64,
        ..Config::vm_fuzzing()
    };
    let mut reached = BTreeMap::<&str, u32>::new();
    let mut clean_runs = 0u32;
    for seed in 0..PROGRAMS {
        let Ok(test) = generate(&mut Unstructured::new(&random_bytes(seed, 16 * 1024)), &cfg)
        else {
            continue;
        };
        if !test.injected_violations.is_empty() || test.post_process.is_some() {
            continue;
        }
        assert_verifies(&test, &format!("in-place program {seed}"));
        let text = test.render();
        for op in [
            "vec_push_back",
            "vec_pop_back",
            "vec_swap",
            "vec_mut_borrow",
            "mut_borrow_variant_field",
            "0x1::vector::",
        ] {
            *reached.entry(op).or_insert(0) += text.matches(op).count() as u32;
        }
        let output = run_output(&test, &format!("program {seed}"));
        clean_runs += output
            .split("\ntask ")
            .skip(1)
            .filter(|task| task.contains(" run") && !task.contains("Error:"))
            .count() as u32;
    }
    for (op, count) in &reached {
        assert!(*count > 0, "no program used {op}");
    }
    assert!(clean_runs > 0, "no run step completed");
}

/// Names of the key structs in `test` with a field that is not a primitive or type parameter.
fn rich_resources(test: &MasmTest) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for step in &test.steps {
        let Step::Publish(module) = step else {
            continue;
        };
        for decl in &module.structs {
            let StructLayout::Singleton(fields) = &decl.layout else {
                continue;
            };
            let rich = fields
                .iter()
                .any(|field| matches!(field.ty, Type::Vector(_) | Type::Struct(_, _)));
            if rich && decl.abilities.has_key() {
                names.insert(format!("::{}::{}", module.id.name(), decl.name));
            }
        }
    }
    names
}

/// Resources with vector, struct and enum fields verify, are stored, and are written back.
#[test]
fn rich_resources_are_stored() {
    let mut stored = 0u32;
    for seed in 0..PROGRAMS {
        let Some(test) = valid_program(seed) else {
            continue;
        };
        let rich = rich_resources(&test);
        if rich.is_empty() {
            continue;
        }
        assert_verifies(&test, &format!("rich-field program {seed}"));
        let output = run_output(&test, &format!("rich-field program {seed}"));
        stored += output
            .lines()
            .filter(|line| line.starts_with("write new "))
            .filter(|line| rich.iter().any(|name| line.contains(&format!("{name} "))))
            .count() as u32;
    }
    eprintln!("stored {stored} resources with rich fields");
    assert!(stored > 0, "no resource with a rich field was stored");
}

/// Programs call the stdlib natives the runner registers, verify, and publish.
#[test]
fn native_calls_verify_and_run() {
    let mut reached = BTreeMap::<&str, u32>::new();
    for seed in 0..PROGRAMS {
        let Some(test) = valid_program(seed) else {
            continue;
        };
        assert_verifies(&test, &format!("native-call program {seed}"));
        let text = test.render();
        for call in [
            "0x1::cmp::compare",
            "0x1::mem::swap",
            "0x1::vector::move_range",
            "0x1::bcs::serialized_size",
        ] {
            *reached.entry(call).or_insert(0) += text.matches(call).count() as u32;
        }
        let output = run_output(&test, &format!("native-call program {seed}"));
        assert!(
            !output.contains("Unable to publish module"),
            "program {seed}: a module failed to publish\n{output}\n{text}"
        );
    }
    eprintln!("native calls: {reached:?}");
    for (call, count) in &reached {
        assert!(*count > 0, "no program called {call}");
    }
}

#[test]
fn type_name_get_replacement_keeps_tests_verifiable() {
    let mut replaced = 0;
    for seed in 0..64 {
        let Some(mut test) = valid_program(seed) else {
            continue;
        };
        if !test.calls_type_name_get() {
            continue;
        }
        assert!(
            test.replace_type_name_get(),
            "seed {seed}: a type_name::get call remains\n{}",
            test.render()
        );
        assert_verifies(&test, &format!("seed {seed}"));
        replaced += 1;
    }
    assert!(replaced > 0, "no program called type_name::get");
}
