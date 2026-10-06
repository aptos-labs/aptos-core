# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# The PR job runs PR code, so it gets no repository secrets. Only users with
# write access can start the dispatch jobs, so they keep them.
class ModuleVerifyWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  PATH = ".github/workflows/module-verify.yaml"

  def source
    File.read(File.join(ROOT, PATH))
  end

  def test_pr_job_runs_without_secrets
    job = jobs(load_workflow("module-verify.yaml")).fetch("test-verify-modules")
    assert_equal "${{ github.event_name == 'pull_request' }}", job.fetch("if")
    refute job.key?("secrets")
  end

  # Named regression: the guard rejects this change whether or not the file is hardened.
  def test_policy_rejects_restoring_secrets_to_the_pr_job
    marker = "  test-verify-modules:\n"
    before, after = source.split(marker, 2)
    head = before + marker + after.sub(/^(    uses: .*\n)/) { "#{Regexp.last_match(1)}    secrets: inherit\n" }
    violations = PrCiPolicy::PolicyChecker.new.check_pair(PATH, source, head)
    assert_includes violations.map(&:job), "test-verify-modules"
  end
end
