// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    evaluation::{EvaluationConfig, FeedbackLevel, InferenceTactic},
    mcp::session::FlowSession,
    tests::common,
};

#[tokio::test]
async fn list_tools_success() {
    let client = common::make_client().await;
    let result = client.list_tools(None).await.expect("list_tools");
    let formatted = common::format_tools_list(&result);
    common::check_baseline(file!(), &formatted);
}

#[tokio::test]
async fn agent_only_omits_wp_and_rejects_direct_call() {
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = client.list_tools(None).await.expect("list_tools");
    assert!(!result
        .tools
        .iter()
        .any(|tool| tool.name == "move_package_wp"));

    let error = common::call_tool_raw(
        &client,
        "move_package_wp",
        serde_json::json!({"package_path": "."}),
    )
    .await
    .expect_err("unregistered WP call must fail at dispatch");
    let message = common::format_service_error(&error);
    assert!(
        message.contains("not found") || message.contains("-32601"),
        "unexpected error: {message}"
    );
}

fn inventory(tactic: InferenceTactic, evaluation_mode: bool) -> Vec<String> {
    inventory_at(tactic, evaluation_mode, FeedbackLevel::Acceptance)
}

fn inventory_at(
    tactic: InferenceTactic,
    evaluation_mode: bool,
    feedback_level: FeedbackLevel,
) -> Vec<String> {
    FlowSession::tool_names(EvaluationConfig {
        inference_tactic: tactic,
        evaluation_mode,
        feedback_level,
    })
}

#[test]
fn tactic_tool_inventories_differ_only_by_wp() {
    // In and out of an evaluation alike: the direct tactic never serves the
    // WP tool, and the two hybrid tactics share one inventory, which is what
    // lets a hybrid plugin offer either per invocation.
    for evaluation_mode in [false, true] {
        let agent = inventory(InferenceTactic::AgentOnly, evaluation_mode);
        let mut guided = inventory(InferenceTactic::HybridGuided, evaluation_mode);
        let flexible = inventory(InferenceTactic::HybridFlexible, evaluation_mode);
        assert_eq!(guided, flexible, "hybrid inventories must be identical");
        assert!(!agent.iter().any(|name| name == "move_package_wp"));
        guided.retain(|name| name != "move_package_wp");
        assert_eq!(agent, guided, "non-WP inventories must be identical");
    }
}

#[test]
fn the_candidate_check_exists_in_every_session() {
    // Testing a specification is ordinary work, not an evaluation apparatus.
    let check = "move_spec_check".to_string();
    for tactic in [
        InferenceTactic::AgentOnly,
        InferenceTactic::HybridGuided,
        InferenceTactic::HybridFlexible,
    ] {
        assert!(inventory(tactic, false).contains(&check));
        assert!(inventory(tactic, true).contains(&check));
    }
    let baseline = inventory_at(InferenceTactic::HybridGuided, true, FeedbackLevel::Baseline);
    assert!(baseline.contains(&check));
}

#[tokio::test]
async fn telemetry_pairs_tool_start_and_end_for_success_and_failure() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("events.jsonl");
    let client = common::make_client_with_telemetry(InferenceTactic::HybridGuided, &path).await;

    let package = common::make_package("telemetry", &[(
        "m.move",
        "module telemetry::m { public fun f(): u64 { 1 } }",
    )]);
    let _ = common::call_tool(
        &client,
        "move_package_status",
        serde_json::json!({"package_path": package.path()}),
    )
    .await;
    let _ = common::call_tool_raw(
        &client,
        "move_package_status",
        serde_json::json!({"package_path": "/path/which/does/not/exist"}),
    )
    .await;

    let records: Vec<serde_json::Value> = std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|record: &serde_json::Value| {
            matches!(record["event"].as_str(), Some("tool_start" | "tool_end"))
        })
        .collect();
    assert_eq!(records.len(), 4);
    for pair in records.chunks_exact(2) {
        assert_eq!(pair[0]["event"], "tool_start");
        assert_eq!(pair[1]["event"], "tool_end");
        assert_eq!(pair[0]["call_id"], pair[1]["call_id"]);
        assert!(pair[1]["duration_us"].as_u64().is_some());
    }
    assert_eq!(records[1]["outcome"], "success");
    assert_eq!(records[3]["outcome"], "rpc_error");
}

#[test]
fn an_evaluation_session_serves_no_network_egress_tool() {
    // A measured session denies Bash, WebFetch and WebSearch so that MCP tools
    // are the only way out of the process. Replay reaches an arbitrary REST
    // endpoint with a caller-supplied bearer token, so serving it would reopen
    // the channel those denials close, and the session's own credential is
    // readable from `/proc/self/environ`.
    let replay = "move_replay_transaction".to_string();
    for tactic in [
        InferenceTactic::AgentOnly,
        InferenceTactic::HybridGuided,
        InferenceTactic::HybridFlexible,
    ] {
        assert!(
            !inventory(tactic, true).contains(&replay),
            "an evaluation session must not serve {replay}"
        );
        assert!(
            inventory(tactic, false).contains(&replay),
            "an ordinary session keeps {replay}"
        );
    }
}

#[tokio::test]
async fn an_evaluation_session_refuses_a_remote_dependency() {
    // With Bash and the web tools denied and replay unserved, package
    // resolution is the last place a session could reach the network: a `git`
    // dependency in a candidate-written manifest would be cloned. It is refused
    // before resolution instead.
    let package = common::make_package("remote", &[("m", "module 0xCAFE::m {}")]);
    let manifest = package.path().join("Move.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    assert!(
        text.contains("[dependencies]\n"),
        "unexpected manifest:\n{text}"
    );
    std::fs::write(
        &manifest,
        text.replace(
            "[dependencies]\n",
            "[dependencies]\nRemote = { git = \"https://example.invalid/r.git\", rev = \"main\", subdir = \".\" }\n",
        ),
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    std::fs::write(
        &config,
        serde_json::to_string(&serde_json::json!({
            "schema_version": 1,
            "package": package.path(),
            "target": "0xCAFE::m",
            "allowed_edit_paths": ["sources/**"],
            "required_contract_categories": ["normal-result"],
            "timeout_seconds": 10,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let error = common::call_tool_raw(
        &client,
        "move_package_status",
        serde_json::json!({ "package_path": package.path() }),
    )
    .await
    .expect_err("a remote dependency must be refused before resolution");
    let message = common::format_service_error(&error);
    assert!(
        message.contains("remote dependency"),
        "unexpected error: {message}"
    );
}

#[tokio::test]
async fn a_spec_check_refuses_a_remote_dependency() {
    // The candidate check builds its own model without going through
    // `resolve_package`, so it has to apply the same refusal itself; it is the
    // tool the workflow reaches for first.
    let package = common::make_package("remote", &[("m", "module 0xCAFE::m {}")]);
    let manifest = package.path().join("Move.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        text.replace(
            "[dependencies]\n",
            "[dependencies]\nRemote = { git = \"https://example.invalid/r.git\", rev = \"main\", subdir = \".\" }\n",
        ),
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    std::fs::write(
        &config,
        serde_json::to_string(&serde_json::json!({
            "schema_version": 1,
            "package": package.path(),
            "target": "0xCAFE::m",
            "allowed_edit_paths": ["sources/**"],
            "required_contract_categories": ["normal-result"],
            "timeout_seconds": 10,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let error = common::call_tool_raw(
        &client,
        "move_spec_check",
        serde_json::json!({ "package_path": package.path() }),
    )
    .await
    .expect_err("the candidate check must refuse a remote dependency before building");
    let message = common::format_service_error(&error);
    assert!(
        message.contains("remote dependency"),
        "unexpected error: {message}"
    );
}

#[tokio::test]
async fn a_transitive_remote_dependency_is_refused() {
    // The resolver follows a local dependency's own manifest, so a root that
    // names only a local dependency could otherwise reach a git dependency one
    // manifest down.
    let package = common::make_package("root", &[("m", "module 0xCAFE::m {}")]);
    let manifest = package.path().join("Move.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        text.replace(
            "[dependencies]\n",
            "[dependencies]\nLocal = { local = \"deps/local\" }\n",
        ),
    )
    .unwrap();
    let local = package.path().join("deps").join("local");
    std::fs::create_dir_all(&local).unwrap();
    std::fs::write(
        local.join("Move.toml"),
        "[package]\nname = \"Local\"\nversion = \"0.0.0\"\n[dependencies]\n\
         Remote = { git = \"https://example.invalid/r.git\", rev = \"main\", subdir = \".\" }\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    std::fs::write(
        &config,
        serde_json::to_string(&serde_json::json!({
            "schema_version": 1, "package": package.path(), "target": "0xCAFE::m",
            "allowed_edit_paths": ["sources/**"], "required_contract_categories": ["normal-result"],
            "timeout_seconds": 10,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let error = common::call_tool_raw(
        &client,
        "move_package_status",
        serde_json::json!({ "package_path": package.path() }),
    )
    .await
    .expect_err("a transitive remote dependency must be refused");
    let message = common::format_service_error(&error);
    assert!(
        message.contains("remote dependency") && message.contains("deps/local"),
        "unexpected error: {message}"
    );
}

#[tokio::test]
async fn a_cached_package_is_rechecked_for_remote_dependencies() {
    // The cache must not answer for a manifest that has since gained a remote
    // dependency: a later filtered rebuild reads the manifest from disk, and
    // watcher invalidation is asynchronous.
    let package = common::make_package("cached", &[("m", "module 0xCAFE::m {}")]);
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    std::fs::write(
        &config,
        serde_json::to_string(&serde_json::json!({
            "schema_version": 1, "package": package.path(), "target": "0xCAFE::m",
            "allowed_edit_paths": ["sources/**"], "required_contract_categories": ["normal-result"],
            "timeout_seconds": 10,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let arguments = serde_json::json!({ "package_path": package.path() });

    // Populate the cache while the manifest is still clean.
    common::call_tool(&client, "move_package_status", arguments.clone()).await;

    let manifest = package.path().join("Move.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        text.replace(
            "[dependencies]\n",
            "[dependencies]\nRemote = { git = \"https://example.invalid/r.git\", rev = \"main\", subdir = \".\" }\n",
        ),
    )
    .unwrap();

    let error = common::call_tool_raw(&client, "move_package_status", arguments)
        .await
        .expect_err("a cache hit must not skip the remote-dependency refusal");
    let message = common::format_service_error(&error);
    assert!(
        message.contains("remote dependency"),
        "unexpected error: {message}"
    );
}
