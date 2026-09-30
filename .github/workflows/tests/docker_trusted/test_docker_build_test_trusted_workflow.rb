# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

class DockerBuildTestTrustedWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  def setup
    @jobs = jobs(load_workflow("docker-build-test-trusted.yaml"))
  end

  # A job that runs steps only to produce outputs costs a runner; it must have a consumer.
  def test_every_step_job_is_needed_by_another_job
    needed = @jobs.values.flat_map { |job| Array(job["needs"]) }
    step_jobs = @jobs.reject { |_name, job| job.key?("uses") }.keys
    assert_empty step_jobs - needed
  end

  def test_reusable_calls_bind_the_pushed_commit
    calls = @jobs.select { |_name, job| job.dig("with", "GIT_SHA") }
    refute_empty calls
    calls.each { |name, job| assert_equal "${{ github.sha }}", job.dig("with", "GIT_SHA"), name }
  end
end
