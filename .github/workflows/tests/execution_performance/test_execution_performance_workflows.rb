# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

module ExecutionPerformanceAssertions
  include WorkflowTestHelper

  # The only job that can read the Slack secret runs one pinned action and no source.
  def assert_slack_job_is_isolated(workflow)
    slack = jobs(workflow).fetch("notify-slack")
    assert_includes slack.fetch("if"), "github.event_name == 'workflow_dispatch'"
    assert_equal({}, slack.fetch("permissions"))
    assert_equal ["slackapi/slack-github-action@"], steps(slack).map { |step| step["uses"].to_s[/\A[^@]+@/] }
    refute steps(slack).any? { |step| step.key?("run") }
    refute_includes slack.to_s, "GIT_SHA"

    secret_jobs = jobs(workflow).select { |_name, job| job.to_s.include?("secrets.") }.keys
    assert_equal ["notify-slack"], secret_jobs
    assert_equal ["EXECUTION_PERF_SLACK_WEBHOOK_URL"], slack.to_s.scan(/secrets\.([A-Z0-9_]+)/).flatten.uniq
    slack
  end
end

class ExecutionPerformanceOrchestratorTests < Minitest::Test
  include ExecutionPerformanceAssertions

  def setup
    @workflow = load_workflow("execution-performance.yaml")
    @jobs = jobs(@workflow)
  end

  def test_pr_orchestration_is_base_owned_revocable_and_pr_scoped
    assert_label_gated_pr_target(@workflow)
  end

  def test_compute_capabilities_share_one_verified_batch_and_keep_distinct_outputs
    compute = @jobs.fetch("compute-authorization")
    assert_equal(
      {
        "approved" => "${{ steps.authorize.outputs.approved }}",
        "e2e_approved" => "${{ fromJSON(steps.authorize.outputs.approvals)['CICD:run-e2e-tests'].approved }}",
        "all_e2e_approved" => "${{ fromJSON(steps.authorize.outputs.approvals)['CICD:run-all-e2e-tests'].approved }}",
        "performance_approved" => "${{ fromJSON(steps.authorize.outputs.approvals)['CICD:run-execution-performance-test'].approved }}",
        "full_approved" => "${{ fromJSON(steps.authorize.outputs.approvals)['CICD:run-execution-performance-full-test'].approved }}",
      },
      compute.fetch("outputs"),
    )

    expected_labels = %w[CICD:run-e2e-tests CICD:run-all-e2e-tests
                         CICD:run-execution-performance-test CICD:run-execution-performance-full-test]
    authorize = assert_compute_authorization(compute)
    assert_equal expected_labels, JSON.parse(authorize.fetch("with").fetch("required_labels"))

    pr_job = @jobs.fetch("execution-performance")
    assert_equal "compute-authorization", pr_job.fetch("needs")
    assert_equal "github.event_name == 'pull_request_target' && needs.compute-authorization.outputs.approved == 'true'",
                 pr_job.fetch("if")
    refute_includes pr_job.to_s, "labels.*.name"
  end

  def test_pr_call_binds_fork_repository_exact_sha_and_secretless_runner
    job = @jobs.fetch("execution-performance")
    assert_equal "./.github/workflows/workflow-run-execution-performance.yaml", job.fetch("uses")
    assert_equal({"contents" => "read"}, job.fetch("permissions"))
    assert_equal "read", job.fetch("cache-mode")
    assert_equal "${{ github.event.pull_request.head.repo.full_name }}", job.fetch("with").fetch("GIT_REPOSITORY")
    assert_equal "${{ github.event.pull_request.head.sha }}", job.fetch("with").fetch("GIT_SHA")
    assert_equal "${{ github.event.pull_request.base.sha }}", job.fetch("with").fetch("BASE_SHA")
    assert_equal "benchmark-c3d-60-pr", job.fetch("with").fetch("RUNNER_NAME")
    refute job.key?("secrets")
    refute_includes job.to_s, "secrets."
  end

  def test_wrapper_dispatch_binds_the_dispatched_commit
    job = @jobs.fetch("execution-performance-dispatch")
    assert_equal "github.event_name == 'workflow_dispatch'", job.fetch("if")
    assert_equal "./.github/workflows/workflow-run-execution-performance.yaml", job.fetch("uses")
    assert_equal "${{ github.repository }}", job.fetch("with").fetch("GIT_REPOSITORY")
    assert_equal "${{ github.sha }}", job.fetch("with").fetch("GIT_SHA")
    refute job.key?("secrets")
  end

  def test_wrapper_slack_is_dispatch_only_and_never_executes_source
    slack = assert_slack_job_is_isolated(@workflow)
    assert_equal "execution-performance-dispatch", slack.fetch("needs")
    assert_includes slack.fetch("if"), "needs.execution-performance-dispatch.result == 'failure'"
  end
end

class ExecutionPerformanceReusableWorkflowTests < Minitest::Test
  include ExecutionPerformanceAssertions

  def setup
    @workflow = load_workflow("workflow-run-execution-performance.yaml")
    @jobs = jobs(@workflow)
  end

  def test_source_jobs_use_exact_repository_and_sha_without_automatic_authority
    assert_equal({"contents" => "read"}, @workflow.fetch("permissions"))
    assert_equal "${{ (github.event_name == 'pull_request_target' || inputs.RUNNER_NAME == 'benchmark-c3d-60-pr') && 'ubuntu-latest' || '2cpu-gh-ubuntu24-x64' }}",
                 @jobs.fetch("test-target-determinator").fetch("runs-on")
    %w[test-target-determinator single-node-performance].each do |job_name|
      job = @jobs.fetch(job_name)
      %w[secrets. GIT_CREDENTIALS id-token write].each { |token| refute_includes job.to_s, token, job_name }
    end
    checkout = checkout_steps(@jobs.fetch("single-node-performance")).first
    assert_equal "${{ inputs.GIT_REPOSITORY || github.repository }}", checkout.fetch("with").fetch("repository")
    assert_equal "${{ inputs.GIT_SHA }}", checkout.fetch("with").fetch("ref")
    assert_equal false, checkout.fetch("with").fetch("persist-credentials")
    assert_equal true, checkout.fetch("with").fetch("allow-unsafe-pr-checkout")
    assert_equal "${{ inputs.RUNNER_NAME }}", @jobs.fetch("single-node-performance").fetch("runs-on")

    rust_setup = steps(@jobs.fetch("single-node-performance")).find do |step|
      step["uses"] == "aptos-labs/aptos-core/.github/actions/rust-setup@main"
    end
    assert_equal "${{ github.event_name == 'pull_request_target' && format('untrusted-pr-{0}', inputs.GIT_SHA) || '' }}",
                 rust_setup.fetch("with").fetch("ADDITIONAL_KEY")
  end

  # The PR controls nothing the determinator job loads as an action: both local
  # actions come from the exact base checkout, and PR source lives in pr-source.
  def test_target_determinator_loads_actions_only_from_trusted_base
    job = @jobs.fetch("test-target-determinator")
    # Every step runs exactly when the BASE_SHA guard runs.
    guard = steps(job).first
    steps(job).each { |step| assert_equal guard.fetch("if"), step.fetch("if"), step.fetch("name") }
    assert_equal({"BASE_SHA" => "${{ inputs.BASE_SHA }}"}, guard.fetch("env"))
    assert_equal '[[ "$BASE_SHA" =~ ^[0-9a-f]{40}$ ]]', guard.fetch("run").strip

    base = checkout_steps(job)
    assert_equal 1, base.length
    assert_equal({"repository" => "${{ github.repository }}", "ref" => "${{ inputs.BASE_SHA }}",
                  "path" => "trusted-base", "fetch-depth" => 1, "persist-credentials" => false},
                 base.first.fetch("with"))

    source = exact_source_step(job)
    assert_equal "./trusted-base/.github/actions/checkout-exact-pr-source", source.fetch("uses")
    assert_equal({"source_repository" => "${{ inputs.GIT_REPOSITORY || github.repository }}",
                  "source_sha" => "${{ inputs.GIT_SHA }}", "path" => "pr-source", "fetch-depth" => "0"},
                 source.fetch("with"))

    fetch = steps(job).find { |step| step["name"] == "Fetch public base main" }
    assert_equal "pr-source", fetch.fetch("working-directory")

    determinator = steps(job).find { |step| step["id"] == "determine_test_targets" }
    assert_equal "./trusted-base/.github/actions/test-target-determinator", determinator.fetch("uses")
    assert_equal({"working_directory" => "pr-source"}, determinator.fetch("with"))

    refute steps(job).any? { |step| step["uses"].to_s.start_with?("./.github/") }
  end

  def test_target_determinator_action_runs_in_the_given_source_directory
    path = File.join(ROOT, ".github", "actions", "test-target-determinator", "action.yaml")
    action = PrCiPolicy::SafeYaml.load(File.read(path), "test-target-determinator/action.yaml")
    assert_equal ".", action.fetch("inputs").fetch("working_directory").fetch("default")
    action_steps = action.fetch("runs").fetch("steps")
    # PR code runs in this action's directory; no JavaScript action may run next
    # to it, because JavaScript actions receive the Actions runtime token.
    assert_equal [], action_steps.filter_map { |step| step["uses"] }
    action_steps.each do |step|
      assert_equal "${{ inputs.working_directory }}", step.fetch("working-directory"), step.fetch("name")
    end
  end

  def test_workflow_call_interface_is_secretless_and_requires_repository_binding
    call = trigger(@workflow).fetch("workflow_call")
    assert_equal true, call.fetch("inputs").fetch("GIT_REPOSITORY").fetch("required")
    assert_equal true, call.fetch("inputs").fetch("GIT_SHA").fetch("required")
    assert_equal false, call.fetch("inputs").fetch("BASE_SHA").fetch("required")
    refute call.key?("secrets")
    refute call.fetch("inputs").key?("NOTIFY_SLACK")
  end

  def test_direct_manual_slack_is_dispatch_only_and_never_executes_source
    slack = assert_slack_job_is_isolated(@workflow)
    assert_equal %w[test-target-determinator single-node-performance], slack.fetch("needs")
    assert_includes slack.fetch("if"), "inputs.NOTIFY_SLACK"
  end
end
