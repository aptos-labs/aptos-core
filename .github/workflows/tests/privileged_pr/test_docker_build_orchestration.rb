# frozen_string_literal: true

require "minitest/autorun"
require "open3"
require_relative "../workflow_test_helper"

# Structural invariants of docker-build-test.yaml and the Docker actions.
# Docker IDs come from .github/ci/docker-capabilities.json, never from a list here.
class DockerBuildOrchestrationTests < Minitest::Test
  include WorkflowTestHelper
  H = WorkflowTestHelper
  PR_WORKFLOW = "docker-build-test.yaml"
  TRUSTED_WORKFLOW = "docker-build-test-trusted.yaml"
  PLAN = "needs.compute-authorization.outputs.plan"
  IMAGE_JOBS = {
    "pr-rust-images-local" => ["local", "./.github/workflows/workflow-run-docker-rust-build-pr.yaml"],
    "pr-publish-rust-images" => ["publish", "./.github/workflows/workflow-run-docker-rust-publish-pr.yaml"],
  }.freeze
  # Mirrors the repository branch-protection settings, which live outside the
  # repository. A change here must be made in those settings at the same time.
  BRANCH_PROTECTION_CHECKS = %w[
    rust-images node-api-compatibility-tests cli-e2e-tests faucet-tests-main forge-e2e-test
    forge-compat-test forge-framework-upgrade-test forge-consensus-only-perf-test forge-multiregion-test
  ].freeze
  PYTHON_ACTIONS = %w[docker-capability-plan docker-status-plan docker-forge-pr-report].freeze

  def setup
    @manifest = H.docker_manifest
    @workflow = H.load_workflow(PR_WORKFLOW)
    @jobs = H.jobs(@workflow)
    @workloads = @manifest.fetch("capabilities").flat_map { |capability| capability.fetch("workloads") }
    @markers = @workloads.filter_map { |workload| workload["marker"]&.merge("workload" => workload.fetch("id")) }
  end

  def manifest_checks
    [@manifest.fetch("image_check"), *@workloads.flat_map { |workload| workload.fetch("checks") }].uniq
  end

  def steps_using(job, prefix)
    job.fetch("steps").select { |step| step["uses"].to_s.start_with?(prefix) }
  end

  def test_trigger_permissions_and_concurrency_are_pr_scoped
    assert_label_gated_pr_target(@workflow)
    assert_equal ["pull_request_target"], @workflow.fetch("on").keys
    assert_equal "read", @workflow.fetch("cache-mode")
  end

  def test_trusted_workflow_is_physically_separate
    trusted = H.load_workflow(TRUSTED_WORKFLOW)
    assert_equal %w[push workflow_dispatch], trusted.fetch("on").keys.sort
    assert_empty @jobs.keys & H.jobs(trusted).keys
    trusted_source = File.read(File.join(H::ROOT, ".github", "workflows", TRUSTED_WORKFLOW))
    %w[github.event.pull_request docker-capability-plan docker-status-plan].each do |text|
      refute_includes trusted_source, text
    end
    @manifest.fetch("capabilities").each { |capability| refute_includes trusted_source, capability.fetch("label") }
  end

  def test_compute_authorization_is_the_only_plan_source
    auth = @jobs.fetch("compute-authorization")
    assert_equal({ "contents" => "read", "pull-requests" => "read" }, auth.fetch("permissions"))
    checkout = steps_using(auth, "actions/checkout@").first
    assert_equal "${{ github.event.pull_request.base.sha }}", checkout.dig("with", "ref")
    assert_equal false, checkout.dig("with", "persist-credentials")
    plan_steps = steps_using(auth, "./.github/actions/docker-capability-plan")
    assert_equal 1, plan_steps.length
    assert_equal "plan", plan_steps.first.fetch("id")
    assert_equal({ "pr_number" => "${{ github.event.pull_request.number }}",
                   "protected_runners_enabled" => "${{ vars.PROTECTED_RUNNERS_ENABLED == 'true' }}" },
                 plan_steps.first.fetch("with"))
    assert_equal({ "plan" => "${{ steps.plan.outputs.plan }}" }, auth.fetch("outputs"))

    source = File.read(File.join(H::ROOT, ".github", "workflows", PR_WORKFLOW))
    @manifest.fetch("capabilities").each do |capability|
      refute_includes source, capability.fetch("label")
      refute_includes source, capability.fetch("id")
    end
    @jobs.each do |job_id, job|
      refute_includes job.fetch("if", "").to_s, "labels.*.name", job_id
    end
  end

  def test_image_matrices_consume_plan_includes
    IMAGE_JOBS.each do |job_id, (key, reusable)|
      job = @jobs.fetch(job_id)
      assert_equal [job_id], @jobs.select { |_id, other| other["uses"] == reusable }.keys
      assert_equal "fromJSON(#{PLAN}).#{key}.enabled == true", job.fetch("if"), job_id
      assert_equal "${{ fromJSON(#{PLAN}).#{key}.include }}", job.dig("strategy", "matrix", "include"), job_id
      %w[profile features build_target].each do |field|
        assert_equal "${{ matrix.#{field} }}", job.dig("with", field.upcase), job_id
      end
      assert_equal "${{ github.event.pull_request.head.sha }}", job.dig("with", "SOURCE_SHA"), job_id
      assert_equal "${{ github.event.pull_request.base.sha }}", job.dig("with", "BASE_SHA"), job_id
    end
    local = @jobs.fetch("pr-rust-images-local")
    assert_equal({ "contents" => "read" }, local.fetch("permissions"))
    refute local.key?("secrets")
    refute_includes local.to_s, "id-token"
  end

  def test_each_manifest_workload_is_one_job_gated_by_its_final_plan_flag
    workload_jobs = @jobs.keys.select { |id| id.start_with?("pr-") } - IMAGE_JOBS.keys
    assert_equal @workloads.map { |workload| workload.fetch("id") }.sort, workload_jobs.sort
    @workloads.each do |workload|
      id = workload.fetch("id")
      job = @jobs.fetch(id)
      assert_equal "fromJSON(#{PLAN}).workloads.#{id} == true", job.fetch("if"), id
      assert_equal %w[compute-authorization pr-publish-rust-images], job.fetch("needs"), id
    end
  end

  # Publication compiles its own variants on the protected fleet; a failed local
  # leg must not skip it. Only the status evaluator waits for the local matrix.
  def test_only_the_evaluator_waits_for_local_builds
    assert_equal %w[compute-authorization], @jobs.fetch("pr-publish-rust-images").fetch("needs")
    waiting = @jobs.select { |_id, job| Array(job["needs"]).include?("pr-rust-images-local") }.keys
    assert_equal %w[evaluate-pr-statuses], waiting
  end

  def test_privileged_calls_are_pr_bound_and_secretless
    privileged = @jobs.select { |_id, job| job.dig("permissions", "id-token") == "write" }
    refute_empty privileged
    privileged.each do |job_id, job|
      {
        "SOURCE_REPOSITORY" => "${{ github.event.pull_request.head.repo.full_name }}",
        "SOURCE_SHA" => "${{ github.event.pull_request.head.sha }}",
        "PR_NUMBER" => "${{ github.event.pull_request.number }}",
        "BASE_SHA" => "${{ github.event.pull_request.base.sha }}",
      }.each { |input, value| assert_equal value, job.dig("with", input), "#{job_id}.#{input}" }
      refute job.key?("secrets"), job_id
      refute_includes job.to_s, "secrets.", job_id
    end
  end

  def test_marker_matrix_mirrors_each_active_manifest_marker
    job = @jobs.fetch("forge-report-source")
    assert_equal "${{ matrix.marker }}", job.fetch("name")
    assert_equal "always() && fromJSON(#{PLAN} || '{}').markers.enabled == true", job.fetch("if")
    assert_equal({ "fail-fast" => false, "matrix" => { "include" => "${{ fromJSON(#{PLAN}).markers.include }}" } },
                 job.fetch("strategy"))
    assert_equal({ "SECURE_RESULT" => "${{ needs[matrix.workload].result }}" }, job.fetch("env"))
    assert_equal ["compute-authorization", *@markers.map { |marker| marker.fetch("workload") }].sort, job.fetch("needs").sort
    assert_equal({}, job.fetch("permissions"))
    %w[actions/checkout artifact secrets.].each { |text| refute_includes job.to_s, text }
  end

  def test_required_check_matrix_emits_exactly_the_branch_protection_checks
    job = @jobs.fetch("required-check")
    names = job.dig("strategy", "matrix", "check")
    assert_equal({ "fail-fast" => false, "matrix" => { "check" => names } }, job.fetch("strategy"))
    assert_equal "${{ matrix.check }}", job.fetch("name")
    assert_equal manifest_checks.sort, names.sort
    assert_equal BRANCH_PROTECTION_CHECKS.sort, names.sort
    assert_equal "always()", job.fetch("if")
    assert_equal ["evaluate-pr-statuses"], job.fetch("needs")
    assert_equal({}, job.fetch("permissions"))
    assert_equal({ "STATUS_OK" => "${{ fromJSON(needs.evaluate-pr-statuses.outputs.statuses || '{}')[matrix.check] }}" },
                 job.fetch("env"))
    assert_equal ['test "$STATUS_OK" = "true"'], job.fetch("steps").map { |step| step.fetch("run") }

    # No other job may report a required-check or marker name.
    reserved = names + @markers.map { |marker| marker.fetch("id") }
    (@jobs.keys - %w[required-check forge-report-source]).each do |job_id|
      refute_includes reserved, @jobs.fetch(job_id).fetch("name", job_id), job_id
    end
  end

  def test_evaluator_receives_every_gated_job_result_as_one_json_input
    job = @jobs.fetch("evaluate-pr-statuses")
    expected = ["compute-authorization", *IMAGE_JOBS.keys, *@workloads.map { |workload| workload.fetch("id") }]
    assert_equal expected.sort, job.fetch("needs").sort
    assert_equal "always()", job.fetch("if")
    assert_equal({ "contents" => "read" }, job.fetch("permissions"))
    checkout = steps_using(job, "actions/checkout@").first
    assert_equal "${{ github.event.pull_request.base.sha }}", checkout.dig("with", "ref")
    assert_equal false, checkout.dig("with", "persist-credentials")
    status = steps_using(job, "./.github/actions/docker-status-plan").first
    assert_equal({ "needs" => "${{ toJSON(needs) }}" }, status.fetch("with"))
    assert_equal({ "statuses" => "${{ steps.#{status.fetch("id")}.outputs.statuses }}" }, job.fetch("outputs"))
  end

  # The launcher step is the only consumer of inputs and the only producer of outputs.
  def test_docker_actions_wire_every_input_and_output_through_the_launcher
    PYTHON_ACTIONS.each do |dir|
      action = H.load_action(dir)
      steps = action.fetch("runs").fetch("steps")
      assert_equal 1, steps.length, dir
      step = steps.first
      action.fetch("inputs").each_key do |input|
        assert_equal "${{ inputs.#{input} }}", step.fetch("env").fetch("INPUT_#{input.upcase}"), dir
      end
      action.fetch("outputs").each do |output, spec|
        assert_equal "${{ steps.#{step.fetch("id")}.outputs.#{output} }}", spec.fetch("value"), dir
      end
    end
  end

  def test_docker_workflow_actions_are_protected_runtime
    prefixes = H.policy_manifest.fetch("protected_runtime_prefixes")
    action_files = @jobs.values.flat_map { |job| job.fetch("steps", []) }
                        .map { |step| step["uses"].to_s }
                        .select { |uses| uses.start_with?("./.github/actions/") }
                        .map { |uses| "#{uses.delete_prefix("./")}/action.yml" }
    refute_empty action_files
    action_files.each do |path|
      assert prefixes.any? { |prefix| path.start_with?(prefix) }, "#{path} must be protected runtime"
    end
  end

  def test_capability_mapping_is_not_redeclared_in_docs
    docs = File.read(File.join(H::ROOT, "docker", "IMAGE_TAGGING.md"))
    @manifest.fetch("capabilities").each do |capability|
      refute_includes docs, capability.fetch("label")
      refute_includes docs, capability.fetch("id")
    end
    assert_includes docs, ".github/ci/docker-capabilities.json"
  end

  def test_documented_feature_tags_match_build_script_normalization
    script = File.read(File.join(H::ROOT, "docker", "builder", "docker-bake-rust-all.sh"))
    rule = script.match(/NORMALIZED_FEATURES_LIST=.*?sed -e '([^']+)'/)&.captures&.first
    refute_nil rule, "build script must expose the feature-tag normalization rule"
    docs = File.read(File.join(H::ROOT, "docker", "IMAGE_TAGGING.md"))
    section = docs.match(/### Build features\n(?<section>.*?)\nSource:/m)&.[](:section)
    refute_nil section, "image-tagging documentation must contain the build-feature table"
    documented = section.scan(/^\| `([^`]+)` \| `([^`]+)` \|$/).to_h

    @manifest.fetch("variants").map { |variant| variant.fetch("features") }.reject(&:empty?).uniq.each do |feature|
      normalized, error, status = Open3.capture3("sed", "-e", rule, stdin_data: feature)
      assert status.success?, "failed to normalize #{feature}: #{error}"
      assert_equal normalized, documented.fetch(feature), feature
      assert_includes docs, "`validator:#{normalized}_abc123`", feature
    end
  end
end
