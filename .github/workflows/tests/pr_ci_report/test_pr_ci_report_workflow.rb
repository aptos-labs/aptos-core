require "minitest/autorun"
require_relative "../workflow_test_helper"

# Producer identity comes from .github/ci/pr-ci-report-producers.json.
PRODUCERS = WorkflowTestHelper.report_producers.to_h do |producer|
  [File.basename(producer.fetch("workflow_path")), { job: producer.fetch("job") }]
end.freeze

class ProducerWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  def upload_step(job)
    job.fetch("steps").find { |step| step.fetch("uses", "").start_with?("actions/upload-artifact@") }
  end

  # The policy checker rejects cache-mode: write. This test also keeps `none` from
  # turning off the restores of the shared trusted cache.
  def test_producers_keep_implicit_shared_trusted_cache_restores
    PRODUCERS.each_key do |filename|
      workflow = load_workflow(filename)
      refute workflow.key?("cache-mode"), filename
      jobs(workflow).each { |name, job| refute job.key?("cache-mode"), "#{filename}: #{name}" }
    end
  end

  def test_producer_workflows_pass_the_hardened_policy_checker
    PRODUCERS.each_key do |filename|
      path = ".github/workflows/#{filename}"
      text = File.read(File.join(ROOT, path))
      assert_empty PrCiPolicy::PolicyChecker.new.check_pair(path, nil, text), filename
    end
  end

  def test_pr_jobs_are_label_gated_and_check_out_the_exact_sha
    PRODUCERS.each do |filename, spec|
      workflow = load_workflow(filename)
      assert_label_gated_pr_target(workflow, filename)
      job = jobs(workflow).fetch(spec[:job])
      assert_equal "compute-authorization", job.fetch("needs")
      assert_includes job.fetch("if"), "needs.compute-authorization.outputs.approved == 'true'"
      assert_equal "${{ needs.compute-authorization.outputs.runner }}", job.fetch("runs-on")
      trusted, *others = checkout_steps(job)
      assert_empty others, filename
      assert_equal "trusted-base", trusted.dig("with", "path")
      assert_equal false, trusted.dig("with", "persist-credentials")
      checkout = job.fetch("steps").find { |step| step["uses"] == "./trusted-base/.github/actions/checkout-exact-pr-source" }
      assert_equal "${{ needs.compute-authorization.outputs.source_repository }}", checkout.dig("with", "source_repository")
      assert_equal "${{ needs.compute-authorization.outputs.source_sha }}", checkout.dig("with", "source_sha")
      assert_equal ".", checkout.dig("with", "path")
      assert_operator job.fetch("steps").index(trusted), :<, job.fetch("steps").index(checkout)
      upload = upload_step(job)
      assert_equal "pr-ci-report-v1", upload.fetch("with").fetch("name")
      assert upload.fetch("with").fetch("path").end_with?("/pr-ci-report-v1.json")
      assert_includes upload.fetch("if"), "needs.compute-authorization.outputs.run_mode == 'pr'"
    end
  end

  # The reporter matches these strings against API run and job metadata.
  # A job without a `name` key is reported under its job ID.
  def test_producer_workflows_match_the_manifest
    report_producers.each do |producer|
      path = producer.fetch("workflow_path")
      assert_match %r{\A\.github/workflows/[^/]+\.yaml\z}, path
      workflow = load_workflow(File.basename(path))
      assert_equal producer.fetch("workflow_name"), workflow.fetch("name"), path
      job = jobs(workflow).fetch(producer.fetch("job"))
      refute job.key?("name"), path
      assert_equal 1, steps(job).count { |step| step["name"] == producer.fetch("benchmark_step") }, path
    end
  end

  def test_manual_runs_write_step_summaries
    PRODUCERS.each do |filename, spec|
      job = jobs(load_workflow(filename)).fetch(spec[:job])
      summary = job.fetch("steps").find { |step| step["name"] == "Write trusted manual step summary" }
      assert_includes summary.fetch("if"), "needs.compute-authorization.outputs.run_mode == 'manual'"
      assert_includes summary.fetch("run"), "GITHUB_STEP_SUMMARY"
    end
  end
end

class ConsumerWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  def test_consumer_has_only_report_permissions_and_completed_producers
    workflow = load_workflow("pr-ci-report.yaml")
    assert_equal(
      { "workflows" => report_producers.map { |producer| producer.fetch("workflow_name") }, "types" => ["completed"] },
      workflow.fetch("on").fetch("workflow_run"),
    )
    assert_equal({ "actions" => "read", "contents" => "read" }, workflow.fetch("permissions"))
    report = jobs(workflow).fetch("report")
    assert_equal({ "actions" => "read", "contents" => "read", "pull-requests" => "write" }, report.fetch("permissions"))
    rendered = workflow.to_s
    refute_includes rendered, "pull_request.head"
    refute_includes rendered, "privileged-pr-ci"
    checkout = report.fetch("steps").find { |step| step.fetch("uses", "").start_with?("actions/checkout@") }
    assert_match(/\Aactions\/checkout@[0-9a-f]{40}\z/, checkout.fetch("uses"))
    assert_equal "${{ github.sha }}", checkout.fetch("with").fetch("ref")
    assert_equal false, checkout.fetch("with").fetch("persist-credentials")
    reporter = report.fetch("steps").find { |step| step["uses"] == "./.github/actions/pr-ci-report" }
    comment = report.fetch("steps").find { |step| step.fetch("uses", "").start_with?("marocchino/sticky-pull-request-comment@") }
    assert_includes comment.fetch("if"), "steps.#{reporter.fetch("id")}.outputs.should_comment == 'true'"
  end

  def test_report_action_keeps_its_interface
    action = load_action("pr-ci-report")
    assert_equal %w[run_id], action.fetch("inputs").keys
  end
end
