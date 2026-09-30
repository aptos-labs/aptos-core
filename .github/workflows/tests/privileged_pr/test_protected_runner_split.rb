# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class ProtectedRunnerSplitTests < Minitest::Test
  include WorkflowTestHelper

  def test_protected_jobs_require_rollout_gate_and_distinct_fleets
    {"workflow-run-docker-rust-publish-pr.yaml" => ["publish-images", "build"],
     "workflow-run-forge-pr.yaml" => ["forge", "forge"],
     "workflow-run-pr-e2e-tests.yaml" => ["e2e-tests", "e2e"]}.each do |file, (id, fleet)|
      job = jobs(load_workflow(file)).fetch(id)
      assert_equal "vars.PROTECTED_RUNNERS_ENABLED == 'true'", job.fetch("if")
      assert_equal "runs-on/fleet=aptos-protected-#{fleet}/env=protected", job.fetch("runs-on")
      assert_equal "privileged-pr-ci", job.fetch("environment")
    end
    build = jobs(load_workflow("workflow-run-docker-rust-publish-pr.yaml")).fetch("build-images")
    assert_equal "vars.PROTECTED_RUNNERS_ENABLED == 'true'", build.fetch("if")
    assert_equal "runs-on/fleet=aptos-protected-build/env=protected", build.fetch("runs-on")
    assert_equal({"contents" => "read"}, build.fetch("permissions"))
    refute build.key?("environment")
    refute steps(build).any? { |step| step["uses"].to_s.include?("gcp-registry-auth") }
    assert_equal "build-images", jobs(load_workflow("workflow-run-docker-rust-publish-pr.yaml"))
      .fetch("publish-images").fetch("needs")
  end

  def test_consumers_require_protected_build_and_select_its_artifact_id
    workflow_jobs = jobs(load_workflow("docker-build-test.yaml"))
    docker_manifest.fetch("capabilities").flat_map { |c| c.fetch("workloads") }.each do |workload|
      job = workflow_jobs.fetch(workload.fetch("id"))
      assert_includes job.fetch("needs"), "pr-publish-rust-images"
      assert_equal "${{ needs.pr-publish-rust-images.outputs.#{workload.fetch('image_variant')}_manifest }}",
                   job.dig("with", "IMAGE_MANIFEST_ID")
    end
  end

  def test_consumers_do_not_build_or_download_by_name
    %w[workflow-run-forge-pr.yaml workflow-run-pr-e2e-tests.yaml].each do |file|
      job = jobs(load_workflow(file)).values.first
      source = job.to_s
      %w[docker-bake setup-buildx image_tag_prefix privileged-pr-setup].each { |token| refute_includes source, token }
      download = steps(job).find { |s| s["uses"].to_s.start_with?("actions/download-artifact@") }
      assert_equal "${{ inputs.IMAGE_MANIFEST_ID }}", download.dig("with", "artifact-ids")
      refute download.fetch("with").key?("name")
      verify = steps(job).find { |s| s.dig("with", "mode") == "verify" }
      refute_nil verify
      assert_operator steps(job).index(download), :<, steps(job).index(verify)
    end
  end

  def test_build_exports_variant_specific_artifact_ids_without_overwrite
    workflow = load_workflow("workflow-run-docker-rust-publish-pr.yaml")
    job = jobs(workflow).fetch("publish-images")
    upload = steps(job).find { |s| s["uses"].to_s.start_with?("actions/upload-artifact@") }
    assert_equal false, upload.dig("with", "overwrite")
    assert_equal "error", upload.dig("with", "if-no-files-found")
    %w[release failpoints performance consensus].each do |variant|
      assert_includes job.fetch("outputs").fetch("#{variant}_manifest"), "inputs.VARIANT == '#{variant}'"
      assert workflow.dig("on", "workflow_call", "outputs").key?("#{variant}_manifest")
    end
  end
end
