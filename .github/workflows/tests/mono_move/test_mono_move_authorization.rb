# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# The compute-authorization job of the mono-move producers. Producer job structure
# is covered by pr_ci_report/test_pr_ci_report_workflow.rb.
class MonoMoveAuthorizationTests < Minitest::Test
  include WorkflowTestHelper

  CHECKOUT_PIN = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
  LABELS = WorkflowTestHelper.report_producers.to_h do |producer|
    [File.basename(producer.fetch("workflow_path")), producer.fetch("label")]
  end.freeze
  PR_EVENT = "github.event_name == 'pull_request_target'"

  def compute(file)
    jobs(load_workflow(file)).fetch("compute-authorization")
  end

  def test_pull_requests_run_only_after_the_label_check_passes
    LABELS.each do |file, label|
      job = compute(file)
      assert_equal "${{ github.event_name == 'workflow_dispatch' || steps.authorize.outputs.approved == 'true' }}",
                   job.fetch("outputs").fetch("approved"), file
      checkout, authorize = steps(job)
      assert_equal 2, steps(job).length, file
      assert_equal CHECKOUT_PIN, checkout.fetch("uses"), file
      assert_equal({"ref" => "${{ github.sha }}", "persist-credentials" => false}, checkout.fetch("with"), file)
      assert_equal "./.github/actions/compute-authorized", authorize.fetch("uses"), file
      assert_equal "authorize", authorize.fetch("id"), file
      assert_equal({"pr_number" => "${{ github.event.pull_request.number }}", "required_label" => label},
                   authorize.fetch("with"), file)
      [checkout, authorize].each { |step| assert_equal PR_EVENT, step.fetch("if"), file }
    end
  end

  def test_pr_context_binds_the_exact_head_and_the_pr_runner
    LABELS.each_key do |file|
      outputs = compute(file).fetch("outputs")
      assert_equal "${{ github.event.pull_request.head.repo.full_name || github.repository }}",
                   outputs.fetch("source_repository"), file
      expected_sha = if file == "mono-move-e2e-perf.yaml"
                       "${{ inputs.GIT_SHA || github.event.pull_request.head.sha || github.sha }}"
                     else
                       "${{ github.event.pull_request.head.sha || github.sha }}"
                     end
      assert_equal expected_sha, outputs.fetch("source_sha"), file
      assert_equal "${{ #{PR_EVENT} && 'pr' || 'manual' }}", outputs.fetch("run_mode"), file
      assert outputs.fetch("runner").start_with?("${{ #{PR_EVENT} && 'benchmark-c3d-60-pr' || "), file
    end
  end

  def test_manual_e2e_options_reach_the_benchmark_only_for_dispatch
    outputs = compute("mono-move-e2e-perf.yaml").fetch("outputs")
    assert_equal "${{ github.event_name == 'workflow_dispatch' && inputs.SELF_COMPARE && '1' || '' }}",
                 outputs.fetch("self_compare")
    {"repeats" => "REPEATS", "num_blocks_per_test" => "NUM_BLOCKS_PER_TEST", "build" => "BUILD"}.each do |name, input|
      assert_includes outputs.fetch(name), "inputs.#{input}", name
    end
  end
end
