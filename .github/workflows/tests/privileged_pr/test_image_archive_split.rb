# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class ImageArchiveSplitTest < Minitest::Test
  include WorkflowTestHelper

  def setup
    @workflow = load_workflow("workflow-run-docker-rust-publish-pr.yaml")
    @build = jobs(@workflow).fetch("build-images")
    @publish = jobs(@workflow).fetch("publish-images")
  end

  def test_pr_build_has_no_registry_privilege_or_protected_environment
    assert_equal({"contents" => "read"}, @build.fetch("permissions"))
    refute @build.key?("environment")
    assert_equal "runs-on/fleet=aptos-protected-build/env=protected", @build.fetch("runs-on")
    assert_equal "./trusted-base/.github/actions/checkout-exact-pr-source", exact_source_step(@build).fetch("uses")
    assert steps(@build).any? { |step| step.fetch("run", "").include?("CI=false docker/builder/docker-bake-rust-all.sh") }
    refute steps(@build).any? { |step| step.fetch("uses", "").include?("gcp-registry-auth") }
    assert_equal "export-build", archive_step(@build).dig("with", "mode")
    assert_equal "${{ steps.archive-upload.outputs.artifact-id }}", @build.dig("outputs", "archive_id")
  end

  def test_publisher_downloads_by_id_and_never_checks_out_pr_source
    assert_equal "privileged-pr-ci", @publish.fetch("environment")
    assert_equal({"contents" => "read", "id-token" => "write"}, @publish.fetch("permissions"))
    assert_equal "build-images", @publish.fetch("needs")
    refute exact_source_step(@publish)
    refute steps(@publish).any? { |step| step.to_s.include?("pr-source") }
    download = steps(@publish).find { |step| step.fetch("uses", "").start_with?("actions/download-artifact@") }
    assert_equal "${{ needs.build-images.outputs.archive_id }}", download.dig("with", "artifact-ids")
    assert_equal "publish-build", archive_step(@publish).dig("with", "mode")
    assert steps(@publish).any? { |step| step.fetch("run", "").include?("docker-bake-rust-all.sh forge") }
  end

  private

  def archive_step(job)
    steps(job).find { |step| step.fetch("uses", "").end_with?("/.github/actions/image-archive") }
  end
end
