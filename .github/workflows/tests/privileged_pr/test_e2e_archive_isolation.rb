# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class E2eArchiveIsolationTests < Minitest::Test
  include WorkflowTestHelper

  def setup
    workflow = load_workflow("workflow-run-pr-e2e-tests.yaml")
    @prepare = jobs(workflow).fetch("prepare-images")
    @tests = jobs(workflow).fetch("e2e-tests")
  end

  def test_archive_is_bound_to_producer_artifact_and_exact_identity
    verifier = steps(@prepare).find { |s| s.dig("with", "mode") == "verify" }
    assert_equal "${{ inputs.BASE_SHA }}", verifier.dig("with", "base_sha")
    refute steps(@tests).any? { |s| s.dig("with", "mode") == "verify" }

    prepare = steps(@prepare).find { |s| s.dig("with", "mode") == "prepare-e2e" }
    load = steps(@tests).find { |s| s.dig("with", "mode") == "load-e2e" }
    assert_equal "./trusted-base/.github/actions/image-archive", prepare.fetch("uses")
    assert_equal prepare.fetch("uses"), load.fetch("uses")
    %w[source_repository source_sha base_sha pr_number run_id run_attempt variant artifact_repo].each do |input|
      assert_equal prepare.dig("with", input), load.dig("with", input), input
    end

    upload = steps(@prepare).find { |s| s["uses"].to_s.start_with?("actions/upload-artifact@") }
    assert_equal "protected-e2e-images-${{ github.run_id }}-${{ github.run_attempt }}-${{ inputs.VARIANT }}", upload.dig("with", "name")
    assert_equal false, upload.dig("with", "overwrite")
    assert_equal "error", upload.dig("with", "if-no-files-found")
    assert_equal "${{ steps.upload-e2e-images.outputs.artifact-id }}", @prepare.dig("outputs", "archive_id")
    assert_equal "prepare-images", @tests.fetch("needs")
    archive_download = steps(@tests).find { |s| s["uses"].to_s.start_with?("actions/download-artifact@") }
    assert_equal "${{ needs.prepare-images.outputs.archive_id }}", archive_download.dig("with", "artifact-ids")
    assert_operator steps(@tests).index(archive_download), :<, steps(@tests).index(load)
  end

  def test_pr_test_execution_starts_after_archive_load_and_never_pulls_spec_images
    load = steps(@tests).find { |s| s.dig("with", "mode") == "load-e2e" }
    first_test = steps(@tests).find { |s| s["name"] == "Generate YAML API specification" }
    assert_operator steps(@tests).index(load), :<, steps(@tests).index(first_test)
    %w[YAML JSON].each do |format|
      spec = steps(@tests).find { |s| s["name"] == "Generate #{format} API specification" }
      assert_includes spec.dig("with", "command"), "--pull=never"
      assert_includes spec.dig("with", "command"), "${GCP_DOCKER_ARTIFACT_REPO}/tools:${PR_IMAGE_TAG}"
    end
  end
end
