# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class IndexerProcessorWorkflowTest < Minitest::Test
  include WorkflowTestHelper

  EXACT_SOURCE_ACTION = "./.github/actions/checkout-exact-pr-source"
  APPROVED_SHA = "${{ github.event.pull_request.head.sha || github.sha }}"
  SOURCE_REPOSITORY = "${{ github.event.pull_request.head.repo.full_name || github.repository }}"
  TRUSTED_REF = "${{ github.sha }}"
  PR_DATA = "$GITHUB_WORKSPACE/pr-source/ecosystem/indexer-grpc"

  def setup
    @workflow = load_workflow("indexer-processor-testing.yaml")
    @jobs = jobs(@workflow)
  end

  def job_steps(job_name)
    steps(@jobs.fetch(job_name))
  end

  def runs(job_name)
    job_steps(job_name).filter_map { |step| step["run"] }.join("\n")
  end

  # Asserts that the job has exactly one checkout, of trusted code, and returns its index.
  def assert_trusted_checkout(job_name, path: nil)
    checkouts = checkout_steps(@jobs.fetch(job_name))
    assert_equal 1, checkouts.length, job_name
    assert_equal PINS.fetch(:checkout), checkouts.first.fetch("uses"), job_name
    expected = {"repository" => "${{ github.repository }}", "ref" => TRUSTED_REF, "persist-credentials" => false}
    expected["path"] = path if path
    assert_equal expected, checkouts.first.fetch("with"), job_name
    job_steps(job_name).index(checkouts.first)
  end

  def assert_exact_source_after_trusted_checkout(job_name)
    trusted_index = assert_trusted_checkout(job_name)
    source = exact_source_step(@jobs.fetch(job_name))
    assert_equal EXACT_SOURCE_ACTION, source.fetch("uses"), job_name
    assert_operator trusted_index, :<, job_steps(job_name).index(source), job_name
    assert_equal({"source_repository" => SOURCE_REPOSITORY, "source_sha" => APPROVED_SHA}, source.fetch("with"), job_name)
    build = job_steps(job_name).find { |step| step["name"] == "Build trusted transaction helper and generator" }
    assert_equal ".", build.fetch("working-directory"), job_name
  end

  def refute_artifact_or_cache_steps(job_name)
    job_steps(job_name).filter_map { |step| step["uses"] }.each do |uses|
      refute_match(%r{\Aactions/(upload-artifact|download-artifact|cache)}, uses, job_name)
    end
  end

  def test_pr_orchestration_is_base_owned_label_gated_and_read_only
    triggers = trigger(@workflow)
    refute triggers.key?("pull_request")
    assert_equal %w[labeled unlabeled opened synchronize reopened].sort,
                 triggers.fetch("pull_request_target").fetch("types").sort
    assert_equal({"contents" => "read", "pull-requests" => "read"}, @workflow.fetch("permissions"))
    assert_equal "indexer-processor-${{ github.event.pull_request.number || github.run_id }}",
                 @workflow.fetch("concurrency").fetch("group")
  end

  def test_authorization_fails_closed_for_pull_requests_and_admits_manual_runs
    authorize = @jobs.fetch("authorize")
    assert_equal "${{ github.event_name == 'workflow_dispatch' || steps.compute.outputs.approved == 'true' }}",
                 authorize.fetch("outputs").fetch("approved")
    checkout = job_steps("authorize").first
    assert_equal PINS.fetch(:checkout), checkout.fetch("uses")
    assert_equal TRUSTED_REF, checkout.dig("with", "ref")
    compute = job_steps("authorize").find { |step| step["id"] == "compute" }
    assert_equal "./.github/actions/compute-authorized", compute.fetch("uses")
    assert_equal "CICD:run-indexer-processor-tests", compute.dig("with", "required_label")
    assert_equal "${{ github.event.pull_request.number }}", compute.dig("with", "pr_number")
    [checkout, compute].each { |step| assert_equal "github.event_name == 'pull_request_target'", step.fetch("if") }
    %w[static_validation live_generation].each do |name|
      assert_equal "needs.authorize.outputs.approved == 'true'", @jobs.fetch(name).fetch("if"), name
    end
  end

  def test_static_validation_is_secretless_and_checks_the_exact_source_with_trusted_code
    job = @jobs.fetch("static_validation")
    assert_equal({"contents" => "read"}, job.fetch("permissions"))
    refute job.key?("environment")
    refute_includes job.to_s, "secrets."
    assert_exact_source_after_trusted_checkout("static_validation")
    assert_includes runs("static_validation"),
                    "target/debug/indexer-ci-helper validate-config --config \"#{PR_DATA}/indexer-transaction-generator/imported_transactions/imported_transactions.yaml\""
    refute_artifact_or_cache_steps("static_validation")
    job_steps("static_validation").filter_map { |step| step["uses"] }.each do |uses|
      refute uses.start_with?("google-github-actions/auth@"), uses
      refute uses.start_with?("google-github-actions/get-secretmanager-secrets@"), uses
      refute uses.include?("repository-dispatch"), uses
      refute uses.include?("indexer-processor-dispatch"), uses
    end
  end

  def test_live_generation_runs_only_trusted_binaries_on_the_exact_source_data
    job = @jobs.fetch("live_generation")
    assert_equal %w[authorize static_validation], job.fetch("needs")
    assert_equal "privileged-pr-ci", job.fetch("environment")
    assert_equal({"contents" => "read", "id-token" => "write"}, job.fetch("permissions"))
    assert_equal({"dispatch_required" => "${{ steps.diff_check.outputs.dispatch_required }}"}, job.fetch("outputs"))
    assert_exact_source_after_trusted_checkout("live_generation")

    commands = runs("live_generation").scan(%r{target/debug/[a-z-]+(?: [a-z-]+)?})
    assert_equal(
      [
        "target/debug/indexer-ci-helper materialize-config",
        "target/debug/aptos-indexer-transaction-generator",
        "target/debug/aptos-indexer-transaction-generator",
        "target/debug/indexer-ci-helper compare",
      ],
      commands,
    )
    compare = job_steps("live_generation").find { |step| step["id"] == "diff_check" }
    assert_includes compare.fetch("run"), "--baseline-dir \"#{PR_DATA}/indexer-test-transactions/src/json_transactions\""
    refute_artifact_or_cache_steps("live_generation")
    refute_includes job.to_s, "needs.static_validation.outputs"
    refute job_steps("live_generation").any? { |step| step["uses"].to_s.include?("indexer-processor-dispatch") }
  end

  def test_live_generation_pins_its_cloud_actions_and_passes_keys_only_through_env
    uses = job_steps("live_generation").filter_map { |step| step["uses"] }
    assert_includes uses, PINS.fetch(:gcp_auth)
    assert_includes uses, PINS.fetch(:get_secretmanager_secrets)
    uses.reject { |action| action.start_with?("./") }.each { |action| assert_match(/@[0-9a-f]{40}\z/, action) }

    materialize = job_steps("live_generation").find { |step| step.fetch("run", "").include?("materialize-config") }
    assert_equal(
      {
        "TESTNET_API_KEY_VALUE" => "${{ steps.api_keys.outputs.testnet_api_key }}",
        "MAINNET_API_KEY_VALUE" => "${{ steps.api_keys.outputs.mainnet_api_key }}",
      },
      materialize.fetch("env"),
    )
    @jobs.each_key { |name| refute_includes runs(name), "${{", name }
  end

  def test_live_generation_always_deletes_the_materialized_keys_after_import
    live_steps = job_steps("live_generation")
    import = live_steps.index { |step| step.fetch("run", "").include?("--testing-folder \"$RUNNER_TEMP/indexer-ci-imported-transactions\"") }
    cleanup = live_steps.index { |step| step["name"] == "Delete the materialized API keys" }
    refute_nil import
    refute_nil cleanup
    assert_equal import + 1, cleanup
    assert_equal "always()", live_steps.fetch(cleanup).fetch("if")
    assert_equal 'rm -rf -- "$RUNNER_TEMP/indexer-ci-imported-transactions"', live_steps.fetch(cleanup).fetch("run").strip
  end

  def test_trusted_dispatch_runs_base_owned_code_and_binds_the_approved_sha
    job = @jobs.fetch("trusted_dispatch")
    assert_equal ["live_generation"], job.fetch("needs")
    assert_equal "needs.live_generation.outputs.dispatch_required == 'true'", job.fetch("if")
    assert_equal "privileged-pr-ci", job.fetch("environment")
    assert_equal({"contents" => "read"}, job.fetch("permissions"))
    assert_trusted_checkout("trusted_dispatch", path: ".trusted-ci")
    dispatch = job_steps("trusted_dispatch").find { |step| step["uses"] == "./.trusted-ci/.github/actions/indexer-processor-dispatch" }
    assert_equal APPROVED_SHA, dispatch.dig("with", "approved_sha")
    assert_equal "${{ github.event.pull_request.number || 0 }}", dispatch.dig("with", "pr_number")
    assert_equal "${{ secrets.INDEXER_PROCESSOR_DISPATCH_TOKEN }}", dispatch.dig("with", "token")
    assert_equal "${{ github.run_id }}", dispatch.dig("with", "source_run_id")
    assert_equal "${{ github.run_attempt }}", dispatch.dig("with", "source_run_attempt")
    refute dispatch.fetch("with").key?("commit_hash")
    refute_includes job.to_s, "head.repo.full_name"
    refute_includes job.to_s, "needs.authorize"
    refute job_steps("trusted_dispatch").any? { |step| step["uses"].to_s.end_with?("@main") }
    refute_artifact_or_cache_steps("trusted_dispatch")
    refute job.key?("working-directory")
    job_steps("trusted_dispatch").each do |step|
      refute step.key?("run"), step.fetch("name")
      refute step.key?("working-directory"), step.fetch("name")
    end
  end

  def test_every_job_names_the_approved_source_one_way
    rendered = @workflow.to_s
    assert_equal ["github.event.pull_request.head.sha || github.sha"],
                 rendered.scan(/github\.event\.pull_request\.head\.sha[^}]*/).map(&:strip).uniq
    refute_includes rendered, "needs.target"
  end

  def test_workflow_does_not_print_secret_bearing_configuration
    all_runs = @jobs.keys.map { |name| runs(name) }.join("\n")
    refute_includes all_runs, "set -x"
    refute all_runs.lines.any? { |line| line.include?("cat ") && line.include?("imported_transactions.yaml") }
    refute all_runs.lines.any? { |line| line.include?("sed") && line.include?("api_key") }
    refute all_runs.lines.any? { |line| line.include?("echo") && line.match?(/api_key|dispatch_token/i) }
  end
end
