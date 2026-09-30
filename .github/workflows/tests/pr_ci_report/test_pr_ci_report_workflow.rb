require "minitest/autorun"
require_relative "../workflow_test_helper"

# Producer identity comes from .github/ci/pr-ci-report-producers.json.
PRODUCERS = WorkflowTestHelper.report_producers.to_h do |producer|
  [File.basename(producer.fetch("workflow_path")), { job: producer.fetch("job"), label: producer.fetch("label") }]
end.freeze

class ProducerWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  def upload_step(job)
    job.fetch("steps").find { |step| step.fetch("uses", "").start_with?("actions/upload-artifact@") }
  end

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

  def test_pr_jobs_are_base_owned_gated_secretless_and_exact_sha
    PRODUCERS.each do |filename, spec|
      workflow = load_workflow(filename)
      events = workflow.fetch("on")
      assert events.key?("workflow_dispatch"), filename
      assert_equal %w[labeled opened reopened synchronize unlabeled],
                   events.fetch("pull_request_target").fetch("types").sort
      assert_equal({ "contents" => "read", "pull-requests" => "read" }, workflow.fetch("permissions"))

      compute = jobs(workflow).fetch("compute-authorization")
      assert_equal "ubuntu-latest", compute.fetch("runs-on")
      authorize = compute.fetch("steps").find { |step| step["uses"] == "./.github/actions/compute-authorized" }
      assert_equal spec[:label], authorize.fetch("with").fetch("required_label"), filename

      job = jobs(workflow).fetch(spec[:job])
      assert_equal "compute-authorization", job.fetch("needs")
      assert_includes job.fetch("if"), "needs.compute-authorization.outputs.approved == 'true'"
      assert_equal "${{ needs.compute-authorization.outputs.runner }}", job.fetch("runs-on")
      checkout = job.fetch("steps").find { |step| step["uses"] == "actions/checkout@v4" }
      assert_equal "${{ needs.compute-authorization.outputs.source_repository }}", checkout.fetch("with").fetch("repository")
      assert_equal "${{ needs.compute-authorization.outputs.source_sha }}", checkout.fetch("with").fetch("ref")
      assert_equal false, checkout.fetch("with").fetch("persist-credentials")
      rendered = job.to_s
      refute_includes rendered, "secrets."
      refute_includes rendered, "pull-requests: write"
      refute_includes rendered, "id-token: write"
      upload = upload_step(job)
      assert_equal "pr-ci-report-v1", upload.fetch("with").fetch("name")
      assert upload.fetch("with").fetch("path").end_with?("/pr-ci-report-v1.json")
      assert_includes upload.fetch("if"), "needs.compute-authorization.outputs.run_mode == 'pr'"
    end
  end

  def test_producer_job_names_and_manual_inputs_match_the_reporter
    PRODUCERS.each { |filename, spec| assert jobs(load_workflow(filename)).key?(spec[:job]), filename }
    assert_equal %w[BUILD NUM_BLOCKS_PER_TEST REPEATS RUNNER_NAME SELF_COMPARE],
                 load_workflow("mono-move-e2e-perf.yaml").fetch("on").fetch("workflow_dispatch").fetch("inputs").keys.sort
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

  def test_compute_job_exports_normalized_context
    PRODUCERS.each_key do |filename|
      compute = jobs(load_workflow(filename)).fetch("compute-authorization")
      %w[run_mode source_repository source_sha runner].each do |name|
        assert compute.fetch("outputs").key?(name), "#{filename}: missing normalized #{name} output"
      end
    end
    outputs = jobs(load_workflow("mono-move-e2e-perf.yaml")).fetch("compute-authorization").fetch("outputs")
    %w[build num_blocks_per_test repeats runner_name self_compare run_source].each do |name|
      assert outputs.key?(name), "missing normalized E2E option #{name}"
    end
    perf = jobs(load_workflow("mono-move-e2e-perf.yaml")).fetch("mono-move-e2e-perf").fetch("steps")
                                                        .find { |step| step["id"] == "perf" }
    assert_equal "${{ needs.compute-authorization.outputs.run_source }}", perf.fetch("env").fetch("RUN_SOURCE")
    assert_equal "${{ needs.compute-authorization.outputs.self_compare }}", perf.fetch("env").fetch("SELF_COMPARE")
  end

  def test_micro_benchmark_stages_the_report_writer_beside_compare
    job = jobs(load_workflow("mono-move-micro-bench.yaml")).fetch("mono-move-micro-bench")
    stage = job.fetch("steps").find { |step| step["name"] == "Stage bench tooling" }
    assert_includes stage.fetch("run"), "pr_ci_report.py"
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
    assert_equal %w[comment_header comment_path pr_number should_comment], action.fetch("outputs").keys.sort
  end
end
