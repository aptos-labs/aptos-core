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
  CHECKOUT_PIN = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
  SETUP_PYTHON_PIN = "actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065"
  INSTALL = "python -m pip install --disable-pip-version-check click==8.3.3 psutil==5.9.8 PyYAML==6.0.2"

  def workflow
    @workflow ||= load_workflow(FILE)
  end

  def job
    jobs(workflow).fetch("forge-unit-tests")
  end

  def test_runs_only_on_pull_request_for_the_lookup_paths
    assert_equal(
      {
        "pull_request" => {
          "paths" => [
            ".github/workflows/#{FILE}",
            "testsuite/determinator.py",
            "testsuite/find_latest_image.py",
            "testsuite/forge.py",
            "testsuite/forge_test.py",
            "testsuite/forge-test-runner-template.yaml",
            "testsuite/test_framework/**",
          ],
        },
      },
      trigger(workflow),
    )
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
    checkout = checkout_steps(job).first
    assert_equal CHECKOUT_PIN, checkout.fetch("uses")
    assert_equal(
      {"ref" => "${{ github.event.pull_request.head.sha }}", "persist-credentials" => false},
      checkout.fetch("with"),
    )
    assert_equal({"python-version" => "3.10"}, steps(job).find { |step| step["uses"] == SETUP_PYTHON_PIN }.fetch("with"))
    run_steps = steps(job).filter_map { |step| step["run"]&.strip }
    assert_includes run_steps, INSTALL
    tests = steps(job).find { |step| step["name"] == "Run the Forge unit tests" }
    assert_equal "testsuite", tests.fetch("working-directory")
    assert_equal "python -m unittest forge_test", tests.fetch("run").strip
    run_steps.each { |script| refute_includes script, "${{" }

    smoke = steps(job).find { |step| step["name"] == "Check that the lookup script loads" }
    assert_equal "testsuite", smoke.fetch("working-directory")
    assert_equal "python find_latest_image.py --help", smoke.fetch("run").strip
    assert_same steps(job).last, smoke
  end

  # Hardening makes the guard report removal of this workflow and adds it to
  # the actionlint set.
  def test_workflow_is_hardened
    assert_includes policy_manifest.fetch("hardened_workflows"), PATH
  end
end
