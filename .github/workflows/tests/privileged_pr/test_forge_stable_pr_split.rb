# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# PR unit tests are secretless; authenticated live lookup uses trusted base code.
class ForgeStablePrSplitTests < Minitest::Test
  include WorkflowTestHelper

  FILE = "forge-stable.yaml"
  PATH = ".github/workflows/#{FILE}"
  PR_ONLY = "${{ github.event_name == 'pull_request_target' }}"
  BASE_SHA = "${{ github.event.pull_request.base.sha }}"
  LOOKUP_STEP = "Run the trusted image lookup against trusted history"

  def workflow
    @workflow ||= load_workflow(FILE)
  end

  def job(name)
    jobs(workflow).fetch(name)
  end

  def analysis
    @analysis ||= PrCiPolicy::WorkflowAnalysis.new(PATH, File.read(File.join(ROOT, PATH)))
  end

  def named_step(job_name, name)
    steps(job(job_name)).find { |step| step["name"] == name } || flunk("#{job_name}: no step #{name}")
  end

  def test_runs_on_pull_request_target_and_dispatch_only
    assert_equal %w[pull_request_target workflow_dispatch], trigger(workflow).keys.sort
  end

  def test_workflow_level_grants_no_secrets_or_write_permissions
    assert_equal({"contents" => "read"}, workflow.fetch("permissions"))
    refute workflow.key?("env")
  end

  # Under pull_request_target, github.ref_name and github.sha name the base
  # branch, so without the PR number every PR would share one group.
  def test_pr_runs_use_a_per_pr_concurrency_group
    assert_includes workflow.dig("concurrency", "group"), "format('pr-{0}', github.event.pull_request.number)"
  end

  def test_live_lookup_job_is_gated_by_the_fixed_environment
    live = job("pr-lookup-live")
    assert_equal PR_ONLY, live.fetch("if")
    refute live.key?("needs")
    assert_equal "privileged-pr-ci", live.fetch("environment")
    assert_equal({"contents" => "read", "id-token" => "write"}, live.fetch("permissions"))
    refute live.key?("secrets")
    refute live.key?("env")
    %w[AWS_ GIT_CREDENTIALS].each { |token| refute_includes live.to_s, token }
    assert_equal %i[id_token_write secret secret], analysis.jobs.fetch("pr-lookup-live").privileges.map(&:kind).sort
  end

  def test_live_lookup_runs_only_trusted_base_code
    live = job("pr-lookup-live")
    live_steps = steps(live)
    assert_equal 1, checkout_steps(live).length
    assert_equal(
      {"ref" => BASE_SHA, "path" => "trusted-base", "persist-credentials" => false},
      checkout_steps(live).first.fetch("with").slice("ref", "path", "persist-credentials"),
    )
    refute live.to_s.include?("pr-source")

    refute live_steps.any? { |step| step["uses"].to_s.start_with?("./pr-source/") }
    live_steps.select { |step| step["uses"].to_s.start_with?("./") }.each do |step|
      assert step.fetch("uses").start_with?("./trusted-base/"), step["name"]
    end
    # An unpinned version installs whatever crane release is newest, with no checksum.
    crane = live_steps.find { |step| step["uses"].to_s.start_with?("imjasonh/setup-crane@") }
    assert_match(/\Av\d+\.\d+\.\d+\z/, crane.dig("with", "version"))

    auth = live_steps.find { |step| step["uses"] == "./trusted-base/.github/actions/gcp-registry-auth" }
    assert_equal false, auth.dig("with", "create_credentials_file")
    assert_equal "trusted-base", named_step("pr-lookup-live", LOOKUP_STEP).fetch("working-directory")
  end

  # Under pull_request_target, a job can write the base branch's Actions cache,
  # so every job a PR event can start must wait for the fixed environment.
  def test_every_pr_target_job_waits_for_the_fixed_environment
    reachable = analysis.jobs.select do |_name, analyzed|
      analyzed.sources.map(&:kind).include?(:pull_request_target_workflow)
    end
    assert_equal ["pr-lookup-live"], reachable.keys
    reachable.each_value { |analyzed| assert analyzed.fixed_environment }
  end

  # The branch output can come from github.head_ref, so it reaches shell only through env.
  def test_forge_stable_passes_the_test_branch_to_shell_through_env
    branch = "${{ steps.determine-test-branch.outputs.BRANCH }}"
    job_steps = steps(job("determine-test-metadata"))

    job_steps.filter_map { |step| step["run"] }.each { |script| refute_includes script, "steps.determine-test-branch" }
    ["Hash the branch", "Write summary"].each do |name|
      assert_equal branch, job_steps.find { |step| step["name"] == name }.dig("env", "BRANCH"), name
    end
  end
end
