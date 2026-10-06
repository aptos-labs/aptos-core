# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# The compute-authorization job of the mono-move producers. Producer job structure
# is covered by pr_ci_report/test_pr_ci_report_workflow.rb.
class MonoMoveAuthorizationTests < Minitest::Test
  include WorkflowTestHelper

  LABELS = WorkflowTestHelper.report_producers.to_h do |producer|
    [File.basename(producer.fetch("workflow_path")), producer.fetch("label")]
  end.freeze
  E2E = "mono-move-e2e-perf.yaml"

  def compute(file)
    jobs(load_workflow(file)).fetch("compute-authorization")
  end

  def test_pull_requests_run_only_after_the_label_check_passes
    LABELS.each do |file, label|
      job = compute(file)
      assert_equal "${{ github.event_name == 'workflow_dispatch' || steps.authorize.outputs.approved == 'true' }}",
                   job.fetch("outputs").fetch("approved"), file
      assert_equal label, assert_compute_authorization(job, file).dig("with", "required_label"), file
    end
  end

  def test_pr_context_binds_the_exact_head_and_the_pr_runner
    LABELS.each_key do |file|
      outputs = compute(file).fetch("outputs")
      assert_equal "${{ github.event.pull_request.head.repo.full_name || github.repository }}",
                   outputs.fetch("source_repository"), file
      expected_sha = if file == E2E
                       "${{ inputs.GIT_SHA || github.event.pull_request.head.sha || github.sha }}"
                     else
                       "${{ github.event.pull_request.head.sha || github.sha }}"
                     end
      assert_equal expected_sha, outputs.fetch("source_sha"), file
      assert_equal "${{ #{PR_TARGET} && 'pr' || 'manual' }}", outputs.fetch("run_mode"), file
      assert outputs.fetch("runner").start_with?("${{ #{PR_TARGET} && 'benchmark-c3d-60-pr' || "), file
    end
  end

  # A PR run gets fixed options; only a manual run reads its dispatch inputs.
  def test_manual_e2e_options_are_dispatch_only_and_reach_the_perf_step
    workflow = load_workflow(E2E)
    inputs = trigger(workflow).fetch("workflow_dispatch").fetch("inputs").keys
    outputs = jobs(workflow).fetch("compute-authorization").fetch("outputs")
    assert_equal "${{ github.event_name == 'workflow_dispatch' && inputs.SELF_COMPARE && '1' || '' }}",
                 outputs.fetch("self_compare")
    (inputs - ["SELF_COMPARE"]).each do |input|
      assert_match(/\A\$\{\{ #{Regexp.escape(PR_TARGET)} && '[\w-]+' \|\| inputs\.#{input} /, outputs.fetch(input.downcase))
    end
    perf = steps(jobs(workflow).fetch("mono-move-e2e-perf")).find { |step| step["id"] == "perf" }
    (inputs.map(&:downcase) + ["run_source"]).each do |name|
      assert_equal "${{ needs.compute-authorization.outputs.#{name} }}", perf.fetch("env").fetch(name.upcase), name
    end
  end
end
