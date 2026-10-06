# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# The Forge lookup unit tests run PR code without approval, so they run on
# pull_request: a read-only token and a cache scope limited to the PR ref. The
# same job under pull_request_target could write the base branch's cache.
class ForgeLookupUnitTestsWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  FILE = "forge-lookup-unit-tests.yaml"
  PATH = ".github/workflows/#{FILE}"

  def workflow
    @workflow ||= load_workflow(FILE)
  end

  def job
    jobs(workflow).fetch("forge-unit-tests")
  end

  def test_runs_only_on_pull_request
    assert_equal ["pull_request"], trigger(workflow).keys
  end

  def test_job_is_secretless_and_cache_free
    assert_equal({"contents" => "read"}, workflow.fetch("permissions"))
    assert_equal ["forge-unit-tests"], jobs(workflow).keys
    refute job.key?("permissions")
    refute job.key?("environment")
    refute job.key?("secrets")
    refute_includes job.to_s, "secrets."
    refute steps(job).any? { |step| step["uses"].to_s.start_with?("actions/cache") }
    analyzed = PrCiPolicy::WorkflowAnalysis.new(PATH, File.read(File.join(ROOT, PATH))).jobs.fetch("forge-unit-tests")
    assert_empty analyzed.privileges.to_a
  end

  def test_runs_the_forge_unit_tests_from_the_exact_pr_head
    checkouts = checkout_steps(job)
    assert_equal 1, checkouts.length
    assert_equal(
      {"ref" => "${{ github.event.pull_request.head.sha }}", "persist-credentials" => false},
      checkouts.first.fetch("with"),
    )
    scripts = steps(job).filter_map { |step| step["run"] }
    installs = scripts.flat_map { |script| script.lines.grep(/pip install/) }
    refute_empty installs
    installs.each { |line| assert_includes line, "--require-hashes", line }
  end
end
