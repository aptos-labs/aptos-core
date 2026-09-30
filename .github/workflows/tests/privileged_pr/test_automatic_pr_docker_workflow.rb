# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class AutomaticPrDockerWorkflowTests < Minitest::Test
  def setup
    @workflow = WorkflowTestHelper.load_workflow("workflow-run-docker-rust-build-pr.yaml")
    @job = WorkflowTestHelper.jobs(@workflow).fetch("build-local-images")
  end

  def test_local_build_is_exact_fork_sha_and_has_no_authority_or_shared_cache
    assert_equal({ "contents" => "read" }, @workflow.fetch("permissions"))
    assert_equal "benchmark-c3d-60-pr", @job.fetch("runs-on")
    steps = @job.fetch("steps")
    trusted_checkout = steps.find { |step| step["uses"].to_s.start_with?("actions/checkout@") }.fetch("with")
    assert_equal({ "repository" => "${{ github.repository }}", "ref" => "${{ inputs.BASE_SHA }}",
                   "path" => "trusted-base", "persist-credentials" => false, "fetch-depth" => 1 },
                 trusted_checkout.slice("repository", "ref", "path", "persist-credentials", "fetch-depth"))
    source_checkout = steps.find { |step| step["uses"] == "./trusted-base/.github/actions/checkout-exact-pr-source" }
    refute_nil source_checkout
    assert_equal "${{ inputs.SOURCE_REPOSITORY }}", source_checkout.dig("with", "source_repository")
    assert_equal "${{ inputs.SOURCE_SHA }}", source_checkout.dig("with", "source_sha")

    source = @workflow.to_s
    %w[secrets. id-token environment GIT_CREDENTIALS cache-from cache-to artifact --push].each do |text|
      refute_includes source, text
    end
    %w[CI=false TARGET_REGISTRY=local].each { |text| assert_includes source, text }
  end
end
