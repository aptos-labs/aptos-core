# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

SLACK_ACTION = "slackapi/slack-github-action@af78098f536edbc4de71162a307590698245be95"

module ExecutionPerformanceAssertions
  include WorkflowTestHelper

  # The only job that can read the Slack secret runs one pinned action and no source.
  def assert_slack_job_is_isolated(workflow)
    slack = jobs(workflow).fetch("notify-slack")
    assert_includes slack.fetch("if"), "github.event_name == 'workflow_dispatch'"
    assert_equal({}, slack.fetch("permissions"))
    assert_equal [SLACK_ACTION], steps(slack).map { |step| step["uses"] }
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
    events = trigger(@workflow)
    refute events.key?("pull_request")
    assert_equal %w[labeled unlabeled opened reopened synchronize].sort,
                 events.fetch("pull_request_target").fetch("types").sort
    assert events.key?("workflow_dispatch")
    assert_equal({"contents" => "read", "pull-requests" => "read"}, @workflow.fetch("permissions"))
    assert_equal "execution-performance-${{ github.event.pull_request.number || github.run_id }}",
                 @workflow.fetch("concurrency").fetch("group")
    assert_equal true, @workflow.fetch("concurrency").fetch("cancel-in-progress")
  end

  def test_compute_capabilities_share_one_verified_batch_and_keep_distinct_outputs
    compute = @jobs.fetch("compute-authorization")
    assert_equal "github.event_name == 'pull_request_target'", compute.fetch("if")
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

    checkout = checkout_steps(compute).first
    assert_equal "${{ github.event.pull_request.base.sha }}", checkout.fetch("with").fetch("ref")
    assert_equal false, checkout.fetch("with").fetch("persist-credentials")

    expected_labels = %w[CICD:run-e2e-tests CICD:run-all-e2e-tests
                         CICD:run-execution-performance-test CICD:run-execution-performance-full-test]
    authorization_steps = steps(compute).select { |step| step["uses"] == "./.github/actions/compute-authorized" }
    assert_equal 1, authorization_steps.length
    assert_equal "authorize", authorization_steps.first.fetch("id")
    assert_equal "${{ github.event.pull_request.number }}", authorization_steps.first.fetch("with").fetch("pr_number")
    assert_equal expected_labels, JSON.parse(authorization_steps.first.fetch("with").fetch("required_labels"))

    pr_job = @jobs.fetch("execution-performance")
    assert_equal "compute-authorization", pr_job.fetch("needs")
    assert_equal "github.event_name == 'pull_request_target' && needs.compute-authorization.outputs.approved == 'true'",
                 pr_job.fetch("if")
    inputs = pr_job.fetch("with")
    assert_equal "${{ needs.compute-authorization.outputs.full_approved == 'true' && 'CONTINUOUS' || 'LAND_BLOCKING' }}",
                 inputs.fetch("FLOW")
    assert_equal "${{ needs.compute-authorization.outputs.all_e2e_approved == 'true' || needs.compute-authorization.outputs.performance_approved == 'true' || needs.compute-authorization.outputs.full_approved == 'true' }}",
                 inputs.fetch("IGNORE_TARGET_DETERMINATION")
    refute_includes pr_job.to_s, "labels.*.name"
  end

  def test_pr_call_binds_fork_repository_exact_sha_and_secretless_runner
    job = @jobs.fetch("execution-performance")
    assert_equal "./.github/workflows/workflow-run-execution-performance.yaml", job.fetch("uses")
    assert_equal({"contents" => "read"}, job.fetch("permissions"))
    assert_equal "${{ github.event.pull_request.head.repo.full_name }}", job.fetch("with").fetch("GIT_REPOSITORY")
    assert_equal "${{ github.event.pull_request.head.sha }}", job.fetch("with").fetch("GIT_SHA")
    assert_equal "benchmark-c3d-60-pr", job.fetch("with").fetch("RUNNER_NAME")
    refute job.key?("secrets")
    refute_includes job.to_s, "secrets."
  end

  def test_wrapper_dispatch_keeps_calibrated_behavior
    job = @jobs.fetch("execution-performance-dispatch")
    assert_equal "github.event_name == 'workflow_dispatch'", job.fetch("if")
    assert_equal "./.github/workflows/workflow-run-execution-performance.yaml", job.fetch("uses")
    assert_equal(
      {
        "GIT_REPOSITORY" => "${{ github.repository }}",
        "GIT_SHA" => "${{ github.sha }}",
        "RUNNER_NAME" => "benchmark-c3d-60",
        "FLOW" => "CONTINUOUS",
        "IGNORE_TARGET_DETERMINATION" => true,
      },
      job.fetch("with"),
    )
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
      checkout = checkout_steps(job).first
      assert_equal "${{ inputs.GIT_REPOSITORY || github.repository }}", checkout.fetch("with").fetch("repository"), job_name
      assert_equal "${{ inputs.GIT_SHA }}", checkout.fetch("with").fetch("ref"), job_name
      assert_equal false, checkout.fetch("with").fetch("persist-credentials"), job_name
      %w[secrets. GIT_CREDENTIALS id-token write].each { |token| refute_includes job.to_s, token, job_name }
    end
    assert_equal "${{ inputs.RUNNER_NAME }}", @jobs.fetch("single-node-performance").fetch("runs-on")

    rust_setup = steps(@jobs.fetch("single-node-performance")).find do |step|
      step["uses"] == "aptos-labs/aptos-core/.github/actions/rust-setup@main"
    end
    assert_equal "${{ github.event_name == 'pull_request_target' && format('untrusted-pr-{0}', inputs.GIT_SHA) || '' }}",
                 rust_setup.fetch("with").fetch("ADDITIONAL_KEY")
  end

  def test_shell_commands_read_values_only_from_env
    @jobs.each do |name, job|
      next unless job.key?("steps")
      steps(job).filter_map { |step| step["run"] }.each { |script| refute_includes script, "${{", name }
    end
  end

  def test_workflow_call_interface_is_secretless_and_requires_repository_binding
    call = trigger(@workflow).fetch("workflow_call")
    assert_equal true, call.fetch("inputs").fetch("GIT_REPOSITORY").fetch("required")
    assert_equal true, call.fetch("inputs").fetch("GIT_SHA").fetch("required")
    refute call.key?("secrets")
    refute call.fetch("inputs").key?("NOTIFY_SLACK")
  end

  def test_direct_manual_slack_is_dispatch_only_and_never_executes_source
    slack = assert_slack_job_is_isolated(@workflow)
    assert_equal %w[test-target-determinator single-node-performance], slack.fetch("needs")
    assert_includes slack.fetch("if"), "inputs.NOTIFY_SLACK"
  end
end
