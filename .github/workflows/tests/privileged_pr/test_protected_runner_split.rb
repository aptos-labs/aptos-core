# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class ProtectedRunnerSplitTests < Minitest::Test
  include WorkflowTestHelper

  def test_protected_jobs_require_rollout_gate_and_distinct_fleets
    [["workflow-run-docker-rust-publish-pr.yaml", "build-images", "build"],
     ["workflow-run-docker-rust-publish-pr.yaml", "publish-images", "build"],
     ["workflow-run-forge-pr.yaml", "forge", "forge"],
     ["workflow-run-pr-e2e-tests.yaml", "prepare-images", "e2e"],
     ["workflow-run-pr-e2e-tests.yaml", "e2e-tests", "e2e"]].each do |file, id, fleet|
      job = jobs(load_workflow(file)).fetch(id)
      assert_equal "vars.PROTECTED_RUNNERS_ENABLED == 'true'", job.fetch("if"), id
      assert_equal "runs-on/fleet=aptos-protected-#{fleet}/env=protected", job.fetch("runs-on"), id
    end
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

  def test_build_exports_variant_specific_artifact_ids_without_overwrite
    workflow = load_workflow("workflow-run-docker-rust-publish-pr.yaml")
    job = jobs(workflow).fetch("publish-images")
    upload = steps(job).find { |s| s["uses"].to_s.start_with?("actions/upload-artifact@") }
    assert_equal false, upload.dig("with", "overwrite")
    assert_equal "error", upload.dig("with", "if-no-files-found")
    docker_manifest.fetch("variants").map { |variant| variant.fetch("id") }.each do |variant|
      assert_includes job.fetch("outputs").fetch("#{variant}_manifest"), "inputs.VARIANT == '#{variant}'"
      assert workflow.dig("on", "workflow_call", "outputs").key?("#{variant}_manifest")
    end
  end

  # The publisher pushes exactly the archive that the PR build exported.
  def test_publisher_pushes_only_the_build_archive
    build, publish = jobs(load_workflow("workflow-run-docker-rust-publish-pr.yaml")).values_at("build-images", "publish-images")
    assert steps(build).any? { |step| step.fetch("run", "").include?("CI=false docker/builder/docker-bake-rust-all.sh") }
    assert_equal "export-build", archive_step(build).dig("with", "mode")
    assert_equal "${{ steps.archive-upload.outputs.artifact-id }}", build.dig("outputs", "archive_id")
    assert_equal "build-images", publish.fetch("needs")
    download = steps(publish).find { |step| step.fetch("uses", "").start_with?("actions/download-artifact@") }
    assert_equal "${{ needs.build-images.outputs.archive_id }}", download.dig("with", "artifact-ids")
    assert_equal "publish-build", archive_step(publish).dig("with", "mode")
    assert steps(publish).any? { |step| step.fetch("run", "").include?("docker-bake-rust-all.sh forge") }
  end

  private

  def archive_step(job)
    steps(job).find { |step| step.fetch("uses", "").end_with?("/.github/actions/image-archive") }
  end
end
