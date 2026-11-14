Reference: `81ad86eb3ee`. Entries: 1170.

| class | entries |
| --- | --- |
| baseline-changed | 149 |
| baseline-added | 30 |
| baseline-removed | 2 |
| fixture-new | 314 |
| source-changed | 10 |
| same | 665 |

| package | entries |
| --- | --- |
| move-prover-lean | 484 |
| verex-e2e-tests | 579 |
| verex-ir | 3 |
| verex-rust-exporter | 104 |

| first seen at | changed entries |
| --- | --- |
| `9d015f41b27` (certified carriers) | 108 |
| `d5b620edea0` (Rust verification) | 359 |
| `78dcaf9df45` (current tip) | 38 |

## baseline-changed (149)

### move-prover-lean (68)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| ModifiesSchemaTest | identical | present | present | 3 → 3 | 9d015f41b27 | message-change: same three errors, the goal now spells the struct as a nominal declaration and names the value `val` instead of `fst`. |
| aborts_if | identical | present | present | 22 → 22 | 78dcaf9df45 | message-change: the residual obligation of `aborts_if_not_pure` is now reported as `no abort when aborts at abort 1` and `ResultShape.finish` gained an `exitSite`. |
| bitwise_features | identical | present | present | 6 → 7 | 9d015f41b27 | carriers-regression: `ensures include == spec_contains(features, feature)` (ensures_2) no longer verifies. |
| bug-17117 | identical | present | present | 3 → 12 | 78dcaf9df45 | carriers-regression: the data invariant `invariant x > 20` now fails where a mutation ends and `get_s_error` times out after 14 leaves at 1000000 heartbeats. |
| bv_aborts | identical | present | present | 4 → 4 | 9d015f41b27 | message-change: only `ResultShape.finish` gained an `exitSite` argument in the printed goal. |
| amm_example | identical | present | present | 22 → 24 | 9d015f41b27 | carriers-regression: `aborts_if false` of `amount_out` and `ensures result <= reserve_out` of two functions no longer verify. |
| behavioral_congruence | identical | present | present | 17 → 20 | 78dcaf9df45 | carriers-regression: option.move's data invariant `invariant vec.length <= 1` now fails where a value is constructed, in `rebuilt` and one more target. |
| behavioral_predicates | identical | present | present | 17 → 17 | 9d015f41b27 | feature: Given frame; two new info lines say `map` and `apply2_generic` fall back to concrete verification because Given interpretation lacks function-value calls. |
| behavioral_soundness | identical | present | present | 10 → 10 | 9d015f41b27 | feature: Given frame; a new info line says `generic_reflection_ensures_are_not_assumed` falls back to concrete verification. |
| behavioral_spliced_congruence | identical | present | present | 14 → 6 | 9d015f41b27 | improvement: the baseline lost errors (14 to 6) and the goal shrank to the nominal spelling of the struct rows. |
| bp_forwarding | identical | present | present | 3 → 6 | 9d015f41b27 | carriers-regression: both `ensures` of `set_values` over the entries' values and keys no longer verify. |
| folds_of | identical | present | present | 9 → 7 | 78dcaf9df45 | carriers-regression: `ensures result == spec_fold$lambda$0(...).total` of `fold_sum` now fails (the loop-invariant failure it replaces is gone). |
| folds_of_callee_ensures | identical | present | present | 9 → 9 | 78dcaf9df45 | carriers-regression: a loop-invariant failure turned into `ensures result == spec_fold$lambda$2(v, new Acc { total := 0 }, v.length).total` of the same target. |
| folds_of_map_intrinsic | identical | present | present | 3 → 14 | 9d015f41b27 | carriers-regression: the loop invariants and the `ensures` of `collect`, `collect_distinct` and `remove_all` no longer verify (3 to 14 error lines). |
| folds_of_ref | identical | present | present | 9 → 11 | 9d015f41b27 | carriers-regression: `ensures result == spec_idx_sum(v, v.length)` and `ensures spec.result[1] == v.length` no longer verify. |
| folds_of_wrapper_mut | identical | present | present | 7 → 11 | 78dcaf9df45 | carriers-regression: both `ensures` of `mirror_keys` and one of `set_both` fail, and `set_both` now burns its 1000000-heartbeat budget after 2002 leaves instead of 304. |
| specialize_generic_caller | identical | present | present | 0 → 0 | 9d015f41b27 | feature: Given frame; a new info line says `count_all` falls back to concrete verification for authored assertions. |
| vector_hofs_for_each | identical | present | present | 9 → 22 | 9d015f41b27 | carriers-regression: the loop invariants and `aborts_if false` of `increment_all`, `scale_all` and `increment_all_inferred` no longer verify (9 to 22 error lines). |
| vector_hofs_mut_receiver | identical | present | present | 3 → 17 | 9d015f41b27 | carriers-regression: the loop invariants and `aborts_if` of `bump_field`, `bump_resource`, `add_index` and more no longer verify (3 to 17 error lines). |
| stored_fun_values | identical | present | present | 23 → 23 | 9d015f41b27 | message-change: same 23 errors, with the nominal spelling of the struct, `MemoryInvariants` renamed `DataInvariants` and `AbortsOf` renamed `AbortsOfEntries`. |
| data_invariant_in_map | identical | present | present | 3 → 5 | 9d015f41b27 | carriers-regression: `no_violation` now fails the data invariant `invariant value != 0` where a mutation ends; the obligation was also renamed from `stored_invariant` to `data_invariant_1`. |
| enum | identical | present | present | 7 → 9 | 78dcaf9df45 | carriers-regression: the data invariant `invariant this.x > 20` now fails where a mutation ends, leaving a residual obligation in the enum's field borrow. |
| enum_invariants | identical | present | present | 3 → 7 | 9d015f41b27 | carriers-regression: `add_one_if_first_ok` and one more target now owe `a data invariant where a mutable parameter's loan ends, at the function's exit`. |
| enum_positional | identical | present | present | 3 → 5 | 78dcaf9df45 | carriers-regression: `test_positional_correct` now fails the data invariant `invariant this.0 > 20` where a mutation ends. |
| fun_field_nested_ability_variants | identical | present | present | 4 → 4 | d5b620edea0 | message-change: the same three `no denotation ... a type argument without values is not carried` errors moved from the verify clause to the call site. |
| fun_type_unused_ctor | identical | present | present | 2 → 2 | d5b620edea0 | message-change: the same `no denotation for mk` error moved from the verify clause to the call site. |
| generic_invariants | identical | present | present | 21 → 23 | 78dcaf9df45 | carriers-regression: the module invariant on `Storage<u64, Bool>` at @0x22 no longer holds for `good`. |
| ghost_field | identical | present | present | 3 → 5 | 78dcaf9df45 | carriers-regression: `ensures result == e` of `enum_pass` no longer verifies. |
| ghost_field_bp | identical | present | present | 3 → 5 | 78dcaf9df45 | carriers-regression: `bp_mk` now fails the data invariants `f : Fn(u64) -> BpS has Copy, Drop` and the behavioral invariant where a value is constructed, and `ensures result` leaves a residual obligation. |
| ghost_field_closure_equality_ext | identical | present | present | 5 → 8 | 9d015f41b27 | carriers-regression: the data invariant `f : Fn(u64) -> u64 has Copy, Drop` now fails where a value is constructed, in `fs_eq` and one more target. |
| ghost_field_intrinsic_map_ops | identical | present | present | 13 → 20 | 78dcaf9df45 | carriers-regression: the `ensures` of `rem`, `borrow_write` and `addall` no longer verify; the continuation obligations were also renamed from `after_call_1_of_X` to `after_X`. |
| ghost_field_invariants | identical | present | present | 13 → 12 | 78dcaf9df45 | improvement: the baseline lost the failure of `invariant h < 100`. |
| intrinsic_map_insertion_order | identical | present | present | 5 → 7 | 9d015f41b27 | carriers-regression: `ensures result` of `self_equal` no longer verifies. |
| intrinsic_map_key_order | identical | present | present | 11 → 12 | 9d015f41b27 | carriers-regression: `ensures n == spec_len(old(m)) + 1` no longer verifies. |
| intrinsic_map_rank | identical | present | present | 3 → 5 | 78dcaf9df45 | carriers-regression: `ensures spec_rank(_m, spec_key_at(_m, _i)) == _i` of `key_at_contained` no longer verifies. |
| intrinsic_map_rank_bulk | identical | present | present | 3 → 16 | 78dcaf9df45 | carriers-regression: `ensures enumerated(m)` fails for `bulk_add`, `bulk_upsert`, `merge`, `merge_disjoint` and `split` (3 to 16 error lines). |
| invariants | identical | present | present | 11 → 13 | 78dcaf9df45 | carriers-regression: the data invariant `invariant greater_one(this.x)` now fails where a mutation ends, in `lifetime_invalid_R_2` and one more target. |
| invariants_resources | identical | present | present | 3 → 5 | 78dcaf9df45 | carriers-regression: `ensures result > 0` of `get` no longer verifies. |
| let | identical | present | present | 6 → 6 | 9d015f41b27 | message-change: the same six errors, with the let-bound values typed `NTy.plainOf Carriers.toPlain` instead of `.carrier`. |
| loop_memory_havoc | identical | present | present | 43 → 54 | 78dcaf9df45 | carriers-regression: `bump_counter`'s caller leaves new residual obligations at the loop step (43 to 54 error lines). |
| loops | identical | present | present | 18 → 18 | d5b620edea0 | message-change: `ResultShape.finish` gained an `exitSite`, a continuation was renumbered and `Comp` is printed as `Proofs.Spec`. |
| macro_verification | identical | present | present | 8 → 7 | 9d015f41b27 | improvement: the baseline lost the failure of `ensures result == x + y + z`. |
| mut_ref | identical | present | present | 3 → 15 | 9d015f41b27 | carriers-regression: `invariant value > 0` now fails for `return_ref_different_root`, `return_ref_different_path`, their callers and more (3 to 15 error lines). |
| references | identical | present | present | 3 → 3 | 9d015f41b27 | message-change: same three errors, with the continuation inlined into the goal, `Term.denote` printed as `Term.denoteIn` and `ResultShape.finish` carrying an `exitSite`. |
| resources | identical | present | present | 3 → 5 | 78dcaf9df45 | carriers-regression: `move_from_addr_to_sender` leaves a residual obligation that the sender's resource is absent. |
| script_incorrect | identical | present | present | 3 → 3 | 9d015f41b27 | feature: Given frame; a new info line says `main` falls back to concrete verification for in-body assumptions. |
| closure_bp_post_sub_pre_only | identical | present | present | 3 → 3 | 9d015f41b27 | message-change: same three errors, with the nominal spelling of the struct, a `continues` case name and the mutable's final value moved into the tuple. |
| generic_spec_fun_old | identical | present | present | 3 → 6 | 78dcaf9df45 | carriers-regression: `ensures balance_increased::<USD>(addr)` and `::<EUR>(addr)` of `increment_both` no longer verify. |
| type_dependent_code | identical | present | present | 13 → 13 | 78dcaf9df45 | message-change: same 13 errors, with the nominal spelling of `0x42::M`, `FrameOf` carrying `[BinderKind.typeArg]` and a Given info line. |
| uninst_global_invariant | identical | present | present | 3 → 3 | 9d015f41b27 | feature: Given frame; four new info lines say `test1`..`test4` fall back to concrete verification for storage. |
| verify_iterator_validity | identical | present | present | 27 → 34 | 78dcaf9df45 | carriers-regression: `ensures result == 3`/`== 2` of `refind_after_invalidate`, `vector_same_element` and `preserved_wrapper_ok` no longer verify; preconditions were also renamed from `at call 1` to `at this call`. |
| verify_table | identical | present | present | 6 → 20 | 78dcaf9df45 | carriers-regression: `after_add` and the `ensures` of `borrow_mut`, `add_R` and `borrow_mut_R` no longer verify (6 to 20 error lines). |
| verify_vector | identical | present | present | 26 → 67 | 9d015f41b27 | carriers-regression: the `ensures` of `verify_model_push_back`, `borrow_mut`, `pop_back`, `swap`, `reverse` and more no longer verify (26 to 67 error lines). |
| Escape | identical | present | present | 4 → 7 | 78dcaf9df45 | carriers-regression: both module invariants bounding `Wrapper<..>` to @0x123 now fail after a write of memory they read, in `initialize`. |
| fun_value_equality | identical | present | present | 17 → 18 | 9d015f41b27 | carriers-regression: one more `ensures result` no longer verifies. |
| fun_value_equality_ext | identical | present | present | 5 → 6 | 9d015f41b27 | carriers-regression: `ensures result` at line 35 no longer verifies. |
| generic_aliasing_all_partitions | identical | present | present | 21 → 21 | 9d015f41b27 | feature: Given frame; ten new info lines say the targets fall back to concrete verification for storage. |
| generic_aliasing_ghost_main | identical | present | present | 5 → 5 | 9d015f41b27 | feature: Given frame; nine new info lines say the targets fall back to concrete verification for storage. |
| generic_aliasing_ghost_pair | identical | present | present | 7 → 7 | 9d015f41b27 | feature: Given frame; two new info lines say `concrete` and `declared` fall back to concrete verification. |
| generic_aliasing_many_resources | identical | present | present | 4 → 4 | 9d015f41b27 | feature: Given frame; two new info lines say `only_at_b` and `in_every_case` fall back to concrete verification. |
| generic_aliasing_write_routes | identical | present | present | 18 → 18 | 9d015f41b27 | feature: Given frame; ten new info lines for storage fall-back, and the continuation obligation renamed `after_call_1_of_remove_r` to `after_remove_r`. |
| mono_after_global_invariant | identical | present | present | 3 → 3 | 9d015f41b27 | feature: Given frame; two new info lines say `put_r` and `0x2::Base::put_b` fall back to concrete verification. |
| performance_200511 | identical | present | present | 5 → 3 | 9d015f41b27 | improvement: the baseline lost the failure of `ensures eq_append(result.guid, ...)` in `new_event_handle_impl`. |
| table_key_encoding_kinds | identical | present | present | 7 → 11 | 9d015f41b27 | carriers-regression: the three `ensures` of `struct_keys` over `spec_contains`/`spec_get` no longer verify. |
| type_param_bug_200228 | identical | present | present | 3 → 3 | 9d015f41b27 | feature: Given frame; a new info line says `type_param_bug` falls back to concrete verification for storage. |
| vector_theory_boogie_array_intern | identical | present | present | 5 → 6 | 9d015f41b27 | carriers-regression: the injectivity `ensures` over `pool.holders` no longer verifies. |
| vector_theory_smt_seq | identical | present | present | 5 → 6 | 9d015f41b27 | carriers-regression: the injectivity `ensures` over `pool.holders` no longer verifies. |
| wp_join_label_cycle | identical | present | present | 3 → 3 | 9d015f41b27 | message-change: same three errors, with the nominal spelling of the struct and the locals typed `NTy.plainOf Carriers.toPlain`. |

### verex-e2e-tests (30)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| AggregatorModel | identical | present | present | 8 → 8 | 9d015f41b27 | feature: Given frame; five new info lines for type-reflection fall-back, and the continuation obligation renamed `after_call_1_of_create_aggregator` to `after_create_aggregator`. |
| Modules | identical | present | present | 2 → 2 | 9d015f41b27 | feature: Given frame; a new info line says `wrap_twice` falls back to concrete verification. |
| GenericHofs | identical | present | present | 2 → 2 | 9d015f41b27 | feature: Given frame; two new info lines say `map` and `apply_both` fall back to concrete verification. |
| Known | identical | present | present | 2 → 2 | 9d015f41b27 | feature: Given frame; three new info lines for data-invariant and function-value fall-back. |
| TupleTargets | identical | present | present | 2 → 1 | d5b620edea0 | improvement: one of the two refusals is gone (a tuple-returning `pair` is now a function value); the other is reworded to `a closure whose rows are not its target's is not carried`. |
| PartialAbortsErrors | identical | present | present | 8 → 8 | d5b620edea0 | message-change: the same eight-line verdict is now reported as `no abort when aborts at op 1` with its `from:` condition instead of a residual obligation. |
| MatchPatterns | identical | present | present | 7 → 6 | 9d015f41b27 | improvement: the baseline lost the failure of `aborts_if false` at line 113. |
| MutableTemporaries | identical | present | present | 2 → 4 | 9d015f41b27 | carriers-regression: `ensures c.value == old(c).value + 1 && result == c` of `read_deref` no longer verifies. |
| AssertionErrors | identical | present | present | 12 → 12 | 9d015f41b27 | feature: Given frame; a new info line says `same` falls back to concrete verification for authored assertions. |
| InvocationStateLabels | identical | present | present | 2 → 3 | 9d015f41b27 | carriers-regression: `aborts_if false` at line 65 no longer verifies. |
| SpecFunctionErrors | changed | present | present | 9 → 9 | d5b620edea0 | message-change: the same nine verdicts; the fixture was reworded to read through a `current` spec function instead of `bump`, so only names and line numbers moved. |
| GenericSpecReads | identical | present | present | 2 → 8 | d5b620edea0 | carriers-regression: `deposit` and `deposit_twice` now leave residual obligations over the resolved nominal resource (2 to 8 error lines). |
| GenericStoredInvariants | identical | present | present | 2 → 2 | 9d015f41b27 | feature: Given frame; four new info lines say the targets fall back to concrete verification for storage. |
| behavior_manual_facts | identical | present | present | 2 → 2 | 9d015f41b27 | message-change: same two errors, with the nominal spelling of the struct, an added `registry` hypothesis and `AbortsOf` renamed `AbortsOfEntries`. |
| behavior_manual_facts.proof | changed | present | present | 2 → 2 | 9d015f41b27 | message-change: same two errors with the nominal spelling; the authored script was updated to `simp_all [vir_denote_norm]`. |
| behavior_nominal_arguments | identical | present | present | 4 → 4 | 9d015f41b27 | message-change: same four errors; the proof sketch now also lists `aborts_if_1`. |
| generic_function_literals | identical | present | present | 4 → 6 | 9d015f41b27 | carriers-regression: `aborts_if aborts_of<function[Fn(T) -> Unit](put::<T>)>(x)` of `via_generic` no longer verifies. |
| generic_invariant_ghosts_false | identical | present | present | 2 → 2 | 9d015f41b27 | feature: Given frame; three new info lines say the targets fall back to concrete verification for storage. |
| generic_invariants_false | identical | present | present | 4 → 4 | 9d015f41b27 | feature: Given frame; three new info lines say the targets fall back to concrete verification for storage. |
| generic_spec_vars | identical | present | present | 4 → 18 | d5b620edea0 | carriers-regression: `mint` and its callers leave residual obligations over the resolved nominal resource (4 to 18 error lines). |
| inlined_start_model | identical | present | present | 2 → 7 | 9d015f41b27 | carriers-regression: `invariant v <= 10` now fails for `walk`, `establishes` and one more target (2 to 7 error lines). |
| proof_determined_instances_errors | identical | present | present | 2 → 2 | 9d015f41b27 | message-change: same two errors, with four bound hypotheses of the indexed elements now shown in the context. |
| proof_determined_instances_errors.proof | identical | present | present | 2 → 2 | 9d015f41b27 | message-change: same two errors, with four bound hypotheses of the indexed elements now shown in the context. |
| proof_steps_errors | identical | present | present | 18 → 17 | 9d015f41b27 | message-change: context hypotheses renamed (`bounds` split into `named_lower`/`named_upper`), locals typed `plainOf`, `requires_1_of_same_at_call_1` renamed `requires_1_of_same`; one error fewer. |
| proof_steps_errors.proof | identical | present | present | 18 → 17 | 9d015f41b27 | message-change: context hypotheses renamed (`bounds` split into `named_lower`/`named_upper`), locals typed `plainOf`, `requires_1_of_same_at_call_1` renamed `requires_1_of_same`; one error fewer. |
| proof_stored_clauses_errors | identical | present | present | 3 → 3 | 9d015f41b27 | message-change: same three errors; the context gained `CarriersNonempty` and lost the `MemoryInvariants` hypothesis. |
| proof_stored_clauses_errors.proof | identical | present | present | 3 → 3 | 9d015f41b27 | message-change: same three errors; the context gained `CarriersNonempty` and lost the `MemoryInvariants` hypothesis. |
| stored_maps_at_frames | identical | present | present | 4 → 9 | 9d015f41b27 | carriers-regression: `after_add` and the `aborts_if`/`ensures` of `register` over the registry's entries no longer verify (4 to 9 error lines). |
| table_stored_resource | identical | present | present | 6 → 8 | 9d015f41b27 | carriers-regression: `write` now owes the data invariant `intrinsic_map` where a mutable reference is passed to a call, an obligation absent at the reference. |
| tuple_equations | identical | present | present | 4 → 17 | 9d015f41b27 | carriers-regression: `after_decode`/`after_name_of` and the `aborts_if`/`ensures` of `unpacked` and `extract` no longer verify (4 to 17 error lines). |

### verex-ir (2)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| CompositionPerformance | identical | present | present | 0 → 0 | d5b620edea0 | carriers-regression: the recorded heartbeat counts rose (e.g. `four_times typed` 4029694 to 4617653); a measurement baseline, not a verdict. |
| DenotePerformance | identical | present | present | 0 → 0 | 78dcaf9df45 | carriers-regression: the recorded heartbeat counts rose across the board (e.g. `classify_primitive typed` 7017235 upward); a measurement baseline, not a verdict. |

### verex-rust-exporter (49)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| abort | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| aggregate | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| array_index | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| array_repeat | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| assert | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| basic | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| bitwise_not | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| boolean_bitwise | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| boolean_ordering | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| borrow_call | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| call | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| character | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| comments | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| control | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| drop | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| enum | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| function_pointer | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| generic_adt | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| generic_const_adt | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| generic_enum | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| generic_identity | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| generic_lifetime_adt | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| integer_cast | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| integer_switch | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| integer_widths | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| loop | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| match_guard | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| method | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| multi_call | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| mutable_reference | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| nested_generic_adt | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| nested_reference | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| never | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| overflowing | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| partial_move | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| reference | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| reference_composite | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| scalar | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| shift | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| signed_division | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| slice | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| slice_from_end | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| string_length | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| string_slice | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| struct | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| subslice | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| u128_max | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| unary | identical | present | present | 0 → 0 | d5b620edea0 | rust |
| variant_move | identical | present | present | 0 → 0 | d5b620edea0 | rust |

## baseline-added (30)

### move-prover-lean (16)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| abort_in_fun | identical | absent | present | - → 8 | 78dcaf9df45 | carriers-regression: `call_abort_generic_struct` now leaves a residual obligation on the equality of two `Carriers.lent` spellings of a generic parameter. |
| bitwise_table | identical | absent | present | - → 15 | 78dcaf9df45 | carriers-regression: the baseline appeared with `after_add` of `0x2::table::add` and the `ensures` of `remove`/`borrow_mut`/`add_R` failing. |
| bitwise_table_mixed_instances | identical | absent | present | - → 3 | 9d015f41b27 | carriers-regression: the baseline appeared with `ensures spec_get(result, 1).a == 2` of `structs` failing. |
| discarded_mut_ref_result | identical | absent | present | - → 3 | 9d015f41b27 | carriers-regression: the baseline appeared with the loop invariant of `run` not preserved by an iteration (invariant_3_step). |
| vector_hofs_fold | identical | absent | present | - → 3 | 9d015f41b27 | carriers-regression: the baseline appeared with the loop invariant of `product_concrete` not preserved by an iteration. |
| vector_hofs_noncopy_receiver | identical | absent | present | - → 3 | 78dcaf9df45 | carriers-regression: the baseline appeared because `clear_all` now exceeds its 50000-heartbeat budget with a whnf timeout. |
| conditional_write_back | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info that `diff_resource_generic` falls back to concrete verification. |
| old_param_err | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info that `split` falls back to concrete verification. |
| old_param_ok | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info that `split` falls back to concrete verification. |
| script | identical | absent | present | - → 5 | 78dcaf9df45 | carriers-regression: the baseline appeared with `aborts_if account.address != @0x1`, `aborts_if exists<Info<Token>>(@0x1)` and `ensures exists<Info<Token>>(@0x1)` of `main` failing. |
| script_provider | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info that `register` falls back to concrete verification. |
| unused_global_invariant | identical | absent | present | - → 3 | 78dcaf9df45 | carriers-regression: the baseline appeared with the module invariant `[suspendable] exists<R3<u64>>(@0x2) ==> exists<R3<u64>>(@0x3)` failing after a call in `call_publish_r3`. |
| generic_aliasing_exact_types | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info that `generic` falls back to concrete verification. |
| generic_aliasing_ghost_params | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the infos for `seven_writes` and `nine_writes_generic`. |
| spec_fun_same_mem_param | identical | absent | present | - → 3 | 78dcaf9df45 | carriers-regression: the baseline appeared with `ensures result == spec_is_xdx::<T>()` of `is_xdx` failing. |
| write_back_local_type_inst | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the infos for `f` and `h`. |

### verex-e2e-tests (14)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| GenericResources | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record that `read_generic`'s given data invariant needs behavioral or storage interpretation. |
| StoredFrames | identical | absent | present | - → 2 | 9d015f41b27 | carriers-regression: the baseline appeared with a residual obligation at the `returns` case of the stored frame at line 22. |
| StoredWriteFrames | identical | absent | present | - → 3 | d5b620edea0 | carriers-regression: the baseline appeared with `after_generic_call` and `ensures global<GenericCounter<u64>>(owner) == old(...)` of `concrete_call` failing. |
| OrderedMap | changed | absent | present | - → 12 | 78dcaf9df45 | carriers-regression: the baseline appeared with whnf timeouts at 180000 and 300000 heartbeats for `add` and `insert`, and `invariant sorted_keys(entries)` failing, although the authored proof was updated. |
| GenericStorage | identical | absent | present | - → 2 | 78dcaf9df45 | carriers-regression: the baseline appeared with `ensures result == exists<Vault<u64>>(address)` of `has_u64_vault` failing. |
| Assertions | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info for `unbox`. |
| DefinedStateLabels | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info for `publish_coin`. |
| FrameReadingInvariants | identical | absent | present | - → 6 | 78dcaf9df45 | carriers-regression: the baseline appeared with `Tactic apply failed` unifying `OwnAtArguments` at `dataOwn.case1` for `publish_echo`'s reader. |
| GenericSpecResourceArguments | identical | absent | present | - → 0 | 9d015f41b27 | feature: Given frame; the baseline appeared only to record the info for `increment_generic`. |
| GlobalBorrows | identical | absent | present | - → 2 | 9d015f41b27 | carriers-regression: the baseline appeared with `ensures result == 6` of `bump_left` failing. |
| TableStoredInvariants | changed | absent | present | - → 3 | 78dcaf9df45 | carriers-regression: `ensures result <= 10` of `nested` fails and both `nested` and `wrong_type` now exceed their 25000-heartbeat budget. |
| CargoFixture | none | absent | present | - → 1 | d5b620edea0 | rust: the baseline of the new Rust crate fixture, recording an unspecified native of its dependency. |
| enum_field_index | identical | absent | present | - → 3 | 78dcaf9df45 | carriers-regression: the baseline appeared with both `ensures` of `set_p` over `self.p` failing. |
| enum_storage_field_borrow | identical | absent | present | - → 2 | 9d015f41b27 | carriers-regression: the baseline appeared with `ensures result == global<Nested>(addr).inner.value` of `nested` failing. |

## baseline-removed (2)

### move-prover-lean (2)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| inline_fun | identical | present | absent | 4 → - | 9d015f41b27 | improvement: the baseline vanished; both loop invariants of `test_filter` now verify. |
| fun_type_arity_injectivity | identical | present | absent | 2 → - | d5b620edea0 | improvement: the baseline vanished; a function value of a tuple-returning `two_out` is now denoted. |

## fixture-new (314)

### move-prover-lean (11)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| result_of_transition | new | absent | present | - → 2 | 9d015f41b27 | feature: result_of over a callee's transition, added by 012137ad808 (spec inference); its VIR-SEMANTIC-TYPE message reads oddly (`expected 3 value operand(s), got 3`). |
| result_of_transition_builtins | new | absent | present | - → 7 | 9d015f41b27 | feature: result_of over a callee's transition, added by 012137ad808; the recorded failures are the deliberate `ensures false` targets. |
| result_of_transition_exists_at | new | absent | present | - → 7 | 9d015f41b27 | feature: result_of over a callee's transition, added by 012137ad808; the recorded failures are the deliberate `bad`/`caller_false` targets. |
| result_of_transition_let_post | new | absent | present | - → 5 | 9d015f41b27 | feature: result_of over a callee's transition, added by 012137ad808; the recorded failures are the deliberate `*_false` targets. |
| result_of_transition_let_post_quant | new | absent | absent | - | 9d015f41b27 | feature: result_of over a callee's transition, added clean by 012137ad808. |
| result_of_transition_old_bp | new | absent | present | - → 3 | 9d015f41b27 | feature: result_of over a callee's transition, added by 012137ad808; the recorded failure is the deliberate `caller_false`. |
| result_of_transition_reader | new | absent | present | - → 19 | 9d015f41b27 | feature: result_of over a callee's transition, added by 012137ad808; the recorded failures are the deliberate `ensures false` targets. |
| result_of_transition_state_label_let | new | absent | absent | - | 9d015f41b27 | feature: result_of over a callee's transition, added clean by 012137ad808. |
| index_write_under_loan | new | absent | present | - → 6 | 9d015f41b27 | soundness-fix: the new fixture of 9d015f41b27 records `no denotation ... a write to a local a live mutable borrow uses as an element index is not carried by the denotation`. |
| joined_mut_ref_lenders | new | absent | present | - → 9 | 9d015f41b27 | soundness-fix: the new fixture of 9d015f41b27 records `the data invariant ... does not hold where a mutation ends` for its deliberately broken `locals_broken` and `mixed_global_broken`. |
| mut_ref_returned_data_invariant | new | absent | present | - → 15 | 9d015f41b27 | soundness-fix: the new fixture of 9d015f41b27 records `invariant v == 0` owed where a returned mutable reference's loan ends, in `break_and_keep`, `break_and_return` and `return_reborrow_broken`. |

### verex-e2e-tests (250)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| NominalOpaqueArguments | new | absent | absent | - | 9d015f41b27 | feature: nominal quotation; a clean fixture added by 9d015f41b27 for opaque nominal arguments. |
| NominalSerializationArguments | new | absent | absent | - | 9d015f41b27 | feature: nominal quotation; a clean fixture added by 9d015f41b27 for serialization at nominal arguments. |
| FnMutEvidence | new | absent | present | - → 0 | d5b620edea0 | rust: a closure-evidence fixture added by d5b620edea0 for the Rust Fn/FnMut traits. |
| FnOnceEvidence | new | absent | present | - → 0 | d5b620edea0 | rust: a closure-evidence fixture added by d5b620edea0 for the Rust FnOnce trait. |
| ReceiverEscape | new | absent | absent | - | d5b620edea0 | rust: a clean fixture added by d5b620edea0 for a Rust receiver escaping its method. |
| StaticMethodEvidence | new | absent | present | - → 0 | d5b620edea0 | rust: a fixture added by d5b620edea0 for Rust static-method function values. |
| Destructors | new | absent | present | - → 6 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording `no denotation ... a drop at a type with a destructor is not carried` for Rust Drop. |
| NeverCalls | new | absent | absent | - | d5b620edea0 | rust: a clean fixture added by d5b620edea0 for Rust diverging calls. |
| ArrayArguments | new | absent | absent | - | d5b620edea0 | rust: a clean fixture added by d5b620edea0 for Rust array arguments. |
| GenericRowConditions | new | absent | present | - → 2 | 78dcaf9df45 | feature: a generic callee's conditions at its row (137be863dc3); the recorded failure is the deliberate `counted_u64` precondition. |
| ShapeContractCallErrors | new | absent | present | - → 14 | d5b620edea0 | rust: a fixture added by d5b620edea0 for contracts at a shape, a Rust-profile feature. |
| ShapeContractErrors | new | absent | present | - → 11 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording VIR-SHAPE-OVERLAP/VEREX-SHAPE-* for contracts at a shape. |
| ShapeContracts | new | absent | present | - → 0 | d5b620edea0 | rust: a clean fixture added by d5b620edea0 for contracts at a shape. |
| ShapeContractsRendered | new | absent | absent | - | d5b620edea0 | rust: a clean fixture added by d5b620edea0 rendering contracts at a shape. |
| ConditionalHolders | new | absent | present | - → 24 | d5b620edea0 | rust: a fixture added by d5b620edea0 for Rust conditional holders of references; its failures are the deliberate `*_false`/`*_vacuous` targets. |
| DeclaredLifetimeErrors | new | absent | present | - → 20 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording VIR-SEMANTIC-BORROW-RESULT-REGION for Rust declared lifetimes. |
| DeclaredLifetimes | new | absent | present | - → 10 | d5b620edea0 | rust: a fixture added by d5b620edea0 for Rust declared lifetimes; its failures are the deliberate `*_false` targets. |
| KillJoin | new | absent | present | - → 2 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording VIR-SEMANTIC-BORROW-CONFLICT at a join of loan deaths. |
| NestedHolderErrors | new | absent | present | - → 4 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording VIR-SEMANTIC-BORROW-PARTIAL-MOVE/-CONFLICT/-DROP for nested holders. |
| NestedHolders | new | absent | present | - → 74 | d5b620edea0 | rust: a fixture added by d5b620edea0 for Rust nested reference holders; its failures are the deliberate `*_false` targets. |
| SharedViewErrors | new | absent | present | - → 9 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording `mutable borrow targets an immutable place` and borrow conflicts for shared views. |
| ImpureCalls | new | absent | present | - → 24 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording VEREX-SPEC-IMPURE-CALL for specification calls of `&mut`-taking functions. |
| MutationEnds | new | absent | present | - → 14 | 9d015f41b27 | soundness-fix: the new fixture of 9d015f41b27 records `the data invariant invariant count > 0 does not hold where a mutation ends` for its deliberate `zero`/`zero_field`. |
| arithmetic | new | absent | absent | - | d5b620edea0 | rust |
| arithmetic_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| array_arguments | new | absent | absent | - | d5b620edea0 | rust |
| array_arguments_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| arrays | new | absent | absent | - | d5b620edea0 | rust |
| arrays_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| bench_bit_vector_errors | new | absent | present | - → 15 | d5b620edea0 | rust |
| bench_lower_bound_errors | new | absent | present | - → 11 | d5b620edea0 | rust |
| bench_math64_errors | new | absent | present | - → 14 | d5b620edea0 | rust |
| bench_simple_map_errors | new | absent | present | - → 23 | d5b620edea0 | rust |
| bool_guards | new | absent | present | - → 4 | d5b620edea0 | rust: a fixture added by d5b620edea0 for boolean guards; its failures are the deliberate `*_false` targets. |
| box_destructor_errors | new | absent | present | - → 5 | d5b620edea0 | rust |
| boxes | new | absent | absent | - | d5b620edea0 | rust |
| boxes_false | new | absent | present | - → 36 | d5b620edea0 | rust |
| boxes_vacuous | new | absent | present | - → 36 | d5b620edea0 | rust |
| cast_clauses | new | absent | absent | - | d5b620edea0 | rust |
| casts | new | absent | absent | - | d5b620edea0 | rust |
| casts_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| char_casts | new | absent | absent | - | d5b620edea0 | rust |
| characters | new | absent | absent | - | d5b620edea0 | rust |
| characters_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| closure_loop_numbers_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| closure_specs_errors | new | absent | present | - → 18 | d5b620edea0 | rust |
| closures_errors | new | absent | present | - → 37 | d5b620edea0 | rust |
| cond_moves_errors | new | absent | present | - → 42 | d5b620edea0 | rust |
| constants | new | absent | absent | - | d5b620edea0 | rust |
| constants_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| cursor_errors | new | absent | present | - → 25 | d5b620edea0 | rust |
| data_invariant_call_during_loan | new | absent | absent | - | 9d015f41b27 | feature: data invariants as carriers; a clean fixture added by 9d015f41b27. |
| data_invariant_global_mut | new | absent | present | - → 6 | 9d015f41b27 | soundness-fix: the new fixture of 9d015f41b27 records `invariant count > 0` owed where a mutation of a global ends, for its deliberate `zero`/`zero_field`. |
| data_invariant_reads | new | absent | present | - → 2 | 9d015f41b27 | feature: data invariants as carriers; a fixture added by 9d015f41b27 whose failure is the deliberate `read_after_write_false`. |
| death_points_errors | new | absent | present | - → 32 | d5b620edea0 | rust |
| death_points_move_errors | new | absent | present | - → 28 | d5b620edea0 | rust: a fixture added by d5b620edea0 for the death points of moves; its failures are the deliberate `*_false`/`*_vacuous` targets. |
| declared_lifetimes | new | absent | absent | - | d5b620edea0 | rust |
| declared_lifetimes_bound | new | absent | absent | - | d5b620edea0 | rust |
| declared_lifetimes_errors | new | absent | present | - → 16 | d5b620edea0 | rust |
| dependency_call | new | absent | present | - → 1 | d5b620edea0 | rust |
| derive_eq | new | absent | absent | - | d5b620edea0 | rust |
| derive_eq_errors | new | absent | present | - → 7 | d5b620edea0 | rust |
| destructor_errors | new | absent | present | - → 5 | d5b620edea0 | rust |
| diverging_regions | new | absent | absent | - | d5b620edea0 | rust |
| diverging_regions_errors | new | absent | present | - → 15 | d5b620edea0 | rust |
| drops | new | absent | absent | - | d5b620edea0 | rust |
| drops_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| enum_cursor_errors | new | absent | present | - → 8 | d5b620edea0 | rust |
| enum_holders_errors | new | absent | present | - → 42 | d5b620edea0 | rust |
| enums | new | absent | absent | - | d5b620edea0 | rust |
| enums_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| evidence_composition | new | absent | present | - → 2 | d5b620edea0 | rust |
| evidence_composition_errors | new | absent | present | - → 21 | d5b620edea0 | rust |
| fn_forward | new | absent | present | - → 0 | d5b620edea0 | rust |
| fn_forward_errors | new | absent | present | - → 13 | d5b620edea0 | rust |
| fn_forward_multi | new | absent | present | - → 0 | d5b620edea0 | rust |
| fn_forward_tuple | new | absent | present | - → 0 | d5b620edea0 | rust |
| fn_forward_unit | new | absent | present | - → 0 | d5b620edea0 | rust |
| fn_forward_unit_errors | new | absent | present | - → 9 | d5b620edea0 | rust |
| fnonce_adapters | new | absent | present | - → 2 | d5b620edea0 | rust |
| fnonce_adapters_errors | new | absent | present | - → 11 | d5b620edea0 | rust |
| fnonce_forward | new | absent | present | - → 0 | d5b620edea0 | rust |
| fnonce_forward_errors | new | absent | present | - → 15 | d5b620edea0 | rust |
| generic_copy | new | absent | present | - → 0 | d5b620edea0 | rust |
| generic_copy_false | new | absent | present | - → 18 | d5b620edea0 | rust |
| generic_copy_vacuous | new | absent | present | - → 18 | d5b620edea0 | rust |
| generic_references_errors | new | absent | present | - → 64 | d5b620edea0 | rust |
| generic_tags | new | absent | present | - → 2 | d5b620edea0 | rust |
| generic_tags_false | new | absent | present | - → 6 | d5b620edea0 | rust |
| generic_tags_vacuous | new | absent | present | - → 6 | d5b620edea0 | rust |
| generic_views | new | absent | absent | - | d5b620edea0 | rust |
| generic_views_false | new | absent | present | - → 34 | d5b620edea0 | rust |
| generic_views_vacuous | new | absent | present | - → 34 | d5b620edea0 | rust |
| holder_repoint_errors | new | absent | present | - → 3 | d5b620edea0 | rust |
| holder_structs_errors | new | absent | present | - → 20 | d5b620edea0 | rust |
| impl_trait_params | new | absent | absent | - | d5b620edea0 | rust |
| impl_trait_params_false | new | absent | present | - → 10 | d5b620edea0 | rust |
| impl_trait_params_vacuous | new | absent | present | - → 10 | d5b620edea0 | rust |
| impure_global_errors | new | absent | present | - → 1 | d5b620edea0 | rust: a fixture added by d5b620edea0 recording VEREX-SPEC-IMPURE-CALL for a specification call publishing to storage. |
| impure_proof_step_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| impure_spec_call | new | absent | absent | - | d5b620edea0 | rust |
| impure_spec_call_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| let_mut | new | absent | absent | - | d5b620edea0 | rust |
| let_mut_errors | new | absent | present | - → 8 | d5b620edea0 | rust |
| library_benchmark | new | absent | absent | - | d5b620edea0 | rust |
| library_benchmark_errors | new | absent | present | - → 17 | d5b620edea0 | rust |
| library_bool_guards | new | absent | present | - → 2 | d5b620edea0 | rust |
| library_bool_guards_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| library_clone | new | absent | absent | - | d5b620edea0 | rust |
| library_clone_errors | new | absent | present | - → 14 | d5b620edea0 | rust |
| library_coverage | new | absent | absent | - | d5b620edea0 | rust |
| library_drops | new | absent | absent | - | d5b620edea0 | rust |
| library_drops_errors | new | absent | present | - → 7 | d5b620edea0 | rust |
| library_for | new | absent | present | - → 3 | d5b620edea0 | rust |
| library_for_errors | new | absent | present | - → 17 | d5b620edea0 | rust |
| library_for_wide | new | absent | absent | - | d5b620edea0 | rust |
| library_for_wide_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| library_index | new | absent | absent | - | d5b620edea0 | rust |
| library_index_errors | new | absent | present | - → 7 | d5b620edea0 | rust |
| library_index_mut | new | absent | absent | - | d5b620edea0 | rust |
| library_index_mut_false | new | absent | present | - → 6 | d5b620edea0 | rust |
| library_integer_ops | new | absent | present | - → 2 | d5b620edea0 | rust |
| library_integer_ops_errors | new | absent | present | - → 16 | d5b620edea0 | rust |
| library_integers | new | absent | absent | - | d5b620edea0 | rust |
| library_integers_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| library_iter | new | absent | present | - → 3 | d5b620edea0 | rust |
| library_iter_adapters | new | absent | present | - → 50 | d5b620edea0 | rust |
| library_iter_adapters_errors | new | absent | present | - → 61 | d5b620edea0 | rust |
| library_iter_errors | new | absent | present | - → 19 | d5b620edea0 | rust |
| library_iter_mut | new | absent | present | - → 1 | d5b620edea0 | rust |
| library_iter_mut_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| library_option | new | absent | absent | - | d5b620edea0 | rust |
| library_option_errors | new | absent | present | - → 12 | d5b620edea0 | rust |
| library_option_map | new | absent | absent | - | d5b620edea0 | rust |
| library_option_read | new | absent | absent | - | d5b620edea0 | rust |
| library_option_read_errors | new | absent | present | - → 13 | d5b620edea0 | rust |
| library_option_refs | new | absent | absent | - | d5b620edea0 | rust |
| library_option_refs_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| library_owned_utf8 | new | absent | absent | - | d5b620edea0 | rust |
| library_owned_utf8_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| library_range_inclusive | new | absent | absent | - | d5b620edea0 | rust |
| library_range_inclusive_errors | new | absent | present | - → 14 | d5b620edea0 | rust |
| library_result_read | new | absent | absent | - | d5b620edea0 | rust |
| library_result_read_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| library_slice_get_mut | new | absent | present | - → 1 | d5b620edea0 | rust |
| library_slice_get_mut_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| library_slice_read | new | absent | present | - → 2 | d5b620edea0 | rust |
| library_slice_read_errors | new | absent | present | - → 12 | d5b620edea0 | rust |
| library_slice_write | new | absent | absent | - | d5b620edea0 | rust |
| library_slice_write_false | new | absent | present | - → 7 | d5b620edea0 | rust |
| library_spec_functions | new | absent | present | - → 2 | d5b620edea0 | rust |
| library_spec_functions_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| library_string_read | new | absent | absent | - | d5b620edea0 | rust |
| library_string_read_errors | new | absent | present | - → 8 | d5b620edea0 | rust |
| library_strings | new | absent | absent | - | d5b620edea0 | rust |
| library_strings_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| library_utf8 | new | absent | absent | - | d5b620edea0 | rust |
| library_utf8_error | new | absent | absent | - | d5b620edea0 | rust |
| library_utf8_error_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| library_utf8_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| library_utf8_literals | new | absent | absent | - | d5b620edea0 | rust |
| library_utf8_literals_errors | new | absent | present | - → 22 | d5b620edea0 | rust |
| library_vec | new | absent | absent | - | d5b620edea0 | rust |
| library_vec_edit | new | absent | present | - → 6 | d5b620edea0 | rust |
| library_vec_edit_errors | new | absent | present | - → 14 | d5b620edea0 | rust |
| library_vec_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| lifetime_generic_calls | new | absent | absent | - | d5b620edea0 | rust |
| lifetime_generic_calls_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| loop_invariants | new | absent | absent | - | d5b620edea0 | rust |
| loop_invariants_errors | new | absent | present | - → 13 | d5b620edea0 | rust |
| loop_item_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| loop_local_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| loop_rebind | new | absent | present | - → 4 | d5b620edea0 | rust: a fixture added by d5b620edea0 for rebinding in a loop; its failures are the deliberate `*_false`/`*_vacuous` targets. |
| loop_sites | new | absent | present | - → 0 | d5b620edea0 | rust |
| mem_drop | new | absent | absent | - | d5b620edea0 | rust |
| mem_drop_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| mem_drop_false | new | absent | present | - → 10 | d5b620edea0 | rust |
| mem_drop_vacuous | new | absent | present | - → 12 | d5b620edea0 | rust |
| methods | new | absent | absent | - | d5b620edea0 | rust |
| mutable_slice | new | absent | absent | - | d5b620edea0 | rust |
| mutable_slice_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| mutable_subslice_errors | new | absent | present | - → 1 | d5b620edea0 | rust |
| nested_holders_errors | new | absent | present | - → 18 | d5b620edea0 | rust |
| never | new | absent | absent | - | d5b620edea0 | rust |
| never_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| no_abort | new | absent | present | - → 8 | d5b620edea0 | feature: the `no abort` failure clause, added with the Rust work; its failures are the deliberate `divide`/`assert` targets. |
| no_abort.proof | new | absent | present | - → 8 | d5b620edea0 | feature: the `no abort` failure clause, added with the Rust work; the authored proof file records the same deliberate failures. |
| no_panic | new | absent | present | - → 19 | d5b620edea0 | rust |
| no_panic_cycle | new | absent | present | - → 2 | d5b620edea0 | rust |
| opaque_callee_index_errors | new | absent | present | - → 5 | d5b620edea0 | rust |
| ordering | new | absent | absent | - | d5b620edea0 | rust |
| ordering_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| overflow_flags | new | absent | absent | - | d5b620edea0 | rust |
| overflow_flags_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| panic_macros | new | absent | absent | - | d5b620edea0 | rust |
| panic_macros_errors | new | absent | present | - → 42 | d5b620edea0 | rust |
| partial_move_tag | new | absent | absent | - | d5b620edea0 | rust |
| partial_move_tag_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| partial_operations | new | absent | absent | - | d5b620edea0 | rust |
| partial_operations_errors | new | absent | present | - → 5 | d5b620edea0 | rust |
| promoted_constants | new | absent | absent | - | d5b620edea0 | rust |
| question_mark | new | absent | absent | - | d5b620edea0 | rust |
| question_mark_conversion | new | absent | present | - → 1 | d5b620edea0 | rust |
| question_mark_errors | new | absent | present | - → 13 | d5b620edea0 | rust |
| recursive_list_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| references | new | absent | absent | - | d5b620edea0 | rust |
| references_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| refused_call_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| refused_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| refused_locals_errors | new | absent | present | - → 11 | d5b620edea0 | rust |
| refused_signatures_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| return_in_nested_branch | new | absent | absent | - | d5b620edea0 | rust |
| return_in_nested_branch_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| scope_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| scope_kind | new | absent | present | - → 1 | d5b620edea0 | rust |
| scope_unknown | new | absent | present | - → 1 | d5b620edea0 | rust |
| shared_holder | new | absent | absent | - | d5b620edea0 | rust |
| shared_holder_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| shared_views_aggregates | new | absent | absent | - | d5b620edea0 | rust |
| shared_views_aggregates_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| shared_views_aggregates_false | new | absent | present | - → 24 | d5b620edea0 | rust |
| shared_views_aggregates_vacuous | new | absent | present | - → 24 | d5b620edea0 | rust |
| shared_views_errors | new | absent | present | - → 48 | d5b620edea0 | rust |
| slice_extent | new | absent | absent | - | d5b620edea0 | rust |
| slice_extent_errors | new | absent | present | - → 12 | d5b620edea0 | rust |
| static_fn | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_fn_errors | new | absent | present | - → 14 | d5b620edea0 | rust |
| static_fnmut | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_fnmut_errors | new | absent | present | - → 12 | d5b620edea0 | rust |
| static_fnonce | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_fnonce_errors | new | absent | present | - → 8 | d5b620edea0 | rust |
| static_step | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_step_errors | new | absent | present | - → 12 | d5b620edea0 | rust |
| struct_specs | new | absent | absent | - | d5b620edea0 | rust |
| struct_specs_errors | new | absent | present | - → 2 | d5b620edea0 | rust |
| tail_after_drop | new | absent | absent | - | d5b620edea0 | rust |
| tail_after_drop_false | new | absent | present | - → 4 | d5b620edea0 | rust |
| tail_after_drop_vacuous | new | absent | present | - → 4 | d5b620edea0 | rust |
| temporary_reads_errors | new | absent | present | - → 2 | 78dcaf9df45 | feature: carrying the reads of a temporary read more than once (e9913e5326f); its failure is the deliberate `index_then_write_false`. |
| transfer_return_errors | new | absent | present | - → 4 | d5b620edea0 | rust |
| tuple_callbacks | new | absent | present | - → 0 | d5b620edea0 | rust |
| tuple_callbacks_errors | new | absent | present | - → 12 | d5b620edea0 | rust |
| tuple_element_drop_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| tuple_results | new | absent | absent | - | d5b620edea0 | rust |
| tuple_results_errors | new | absent | present | - → 10 | d5b620edea0 | rust |
| two_modules | new | absent | absent | - | d5b620edea0 | rust |
| two_phase_errors | new | absent | present | - → 14 | d5b620edea0 | rust |
| unsize | new | absent | absent | - | d5b620edea0 | rust |
| unsize_errors | new | absent | present | - → 6 | d5b620edea0 | rust |
| zero_sized_locals_errors | new | absent | present | - → 8 | d5b620edea0 | rust |

### verex-ir (1)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| Utf8Performance | new | absent | present | - → 0 | d5b620edea0 | rust: a performance baseline added by d5b620edea0 for the UTF-8 model of the Rust work. |

### verex-rust-exporter (52)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| fn_forward | new | absent | present | - → 0 | d5b620edea0 | rust |
| fnonce_adapters | new | absent | present | - → 0 | d5b620edea0 | rust |
| fnonce_forward | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_fn | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_fnmut | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_fnonce | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_step | new | absent | present | - → 0 | d5b620edea0 | rust |
| associated_aliases | new | absent | present | - → 3 | d5b620edea0 | rust |
| closure_unconstructed | new | absent | absent | - | d5b620edea0 | rust |
| evidence_composition | new | absent | present | - → 0 | d5b620edea0 | rust |
| partial_move_tag | new | absent | present | - → 0 | d5b620edea0 | rust |
| polyfill_api | new | absent | absent | - | d5b620edea0 | rust |
| const_generic | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_generic_definition | new | absent | present | - → 0 | d5b620edea0 | rust |
| fnmut_boundaries | new | absent | present | - → 0 | d5b620edea0 | rust |
| generic_trait_call | new | absent | present | - → 0 | d5b620edea0 | rust |
| local_types | new | absent | present | - → 0 | d5b620edea0 | rust |
| opaque_callee | new | absent | present | - → 0 | d5b620edea0 | rust |
| opaque_constant | new | absent | present | - → 0 | d5b620edea0 | rust |
| opaque_panic | new | absent | present | - → 0 | d5b620edea0 | rust |
| opaque_private_fields | new | absent | present | - → 0 | d5b620edea0 | rust |
| signature_types | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_interior_mutable | new | absent | present | - → 0 | d5b620edea0 | rust |
| static_mutable | new | absent | present | - → 0 | d5b620edea0 | rust |
| boxes | new | absent | present | - → 0 | d5b620edea0 | rust |
| closure_generic_call | new | absent | present | - → 0 | d5b620edea0 | rust |
| closure_items | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_aggregate | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_promoted | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_reference | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_scalar | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_static | new | absent | present | - → 0 | d5b620edea0 | rust |
| const_zero_sized | new | absent | present | - → 0 | d5b620edea0 | rust |
| copy_types | new | absent | present | - → 0 | d5b620edea0 | rust |
| declared_lifetimes | new | absent | present | - → 0 | d5b620edea0 | rust |
| dependency_calls | new | absent | present | - → 0 | d5b620edea0 | rust |
| dependency_projection | new | absent | present | - → 0 | d5b620edea0 | rust |
| dependency_types | new | absent | present | - → 3 | d5b620edea0 | rust |
| destructors | new | absent | present | - → 0 | d5b620edea0 | rust |
| evidence_arguments | new | absent | present | - → 0 | d5b620edea0 | rust |
| generic_index | new | absent | present | - → 0 | d5b620edea0 | rust |
| impl_trait_parameter | new | absent | present | - → 0 | d5b620edea0 | rust |
| modules | new | absent | present | - → 0 | d5b620edea0 | rust |
| mutable_slice | new | absent | present | - → 0 | d5b620edea0 | rust |
| owned_vector | new | absent | present | - → 0 | d5b620edea0 | rust |
| polyfill | new | absent | present | - → 0 | d5b620edea0 | rust |
| reborrow_twice | new | absent | present | - → 0 | d5b620edea0 | rust |
| reference_argument_adt | new | absent | present | - → 0 | d5b620edea0 | rust |
| slice_extent | new | absent | present | - → 0 | d5b620edea0 | rust |
| tuple_callbacks | new | absent | present | - → 0 | d5b620edea0 | rust |
| unsize | new | absent | present | - → 0 | d5b620edea0 | rust |
| unsize_mutable | new | absent | present | - → 0 | d5b620edea0 | rust |

## source-changed (10)

### verex-e2e-tests (10)

| fixture | source | baseline ref | baseline now | errors | first seen | reason |
| --- | --- | --- | --- | --- | --- | --- |
| Quicksort | changed | absent | absent | - | 9d015f41b27 | feature: certified carriers; the authored proof was ported from `Skolems` to `Carriers`/`CarriersNonempty` with an explicit type assignment, and the fixture stays clean. |
| Generics | changed | absent | absent | - | d5b620edea0 | feature: Given frame; the concrete export now takes `identity.givenTransfer` as refinement evidence and the frame binder is spelled `.typeArg`. |
| LoweringErrors | changed | absent | absent | - | d5b620edea0 | rust: the fixture gained a check that each profile rejects the other's failure clause with VEREX-FAILURE-CLAUSE-PROFILE. |
| Addressing | changed | present | present | 0 → 0 | 9d015f41b27 | message-change: only the fixture's doc pointer moved from `doc/plan/proof-language.md` to `doc/designs/proof-language.md`; the fixture stays clean. |
| RecursiveSpecFunctions | changed | absent | absent | - | 9d015f41b27 | feature: certified carriers; the spec function is now applied to `frame.toCarriers frame.type`, and the fixture stays clean. |
| InvariantChains | changed | absent | absent | - | 9d015f41b27 | message-change: `MemoryInvariants` was renamed `DataInvariants` throughout the unit test; the fixture stays clean. |
| TableAllocation | changed | absent | absent | - | 9d015f41b27 | message-change: only the fixture's doc pointer changed section name; the fixture stays clean. |
| TableGlobalWrites | changed | absent | absent | - | 9d015f41b27 | message-change: the fixture's expected text follows the rename of `does not hold for a value written to memory` to `does not hold where a mutation ends` and of `stored_invariant` to `data_invariant_1`. |
| WrappedTables | changed | absent | absent | - | 9d015f41b27 | carriers-regression: `verify take` no longer decides automatically and the fixture had to add an authored script (`verex_prepare`, `simp_all [SpecMap.hasKey_remove]`). |
| Baseline | changed | absent | absent | - | d5b620edea0 | rust: the harness gained whole-crate Rust verification (`verifyCrate`) and an error-fixture guard. |

## same (665)

Not listed: these fixtures carry the reference's expectation.

