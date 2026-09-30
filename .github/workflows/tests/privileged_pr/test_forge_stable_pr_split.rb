# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# Forge Stable's PR lookup is split by trust. The secretless unit tests run in
# forge-lookup-unit-tests.yaml on pull_request, because under
# pull_request_target unapproved PR code could write the base branch's Actions
# cache. This workflow runs from the base branch (pull_request_target), and PR
# code comes only from the exact head SHA in pr-source/. A privileged-pr-ci
# job runs the PR's lookup against the registry, which refuses anonymous
# reads. Credentialed jobs are dispatch-only.
class ForgeStablePrSplitTests < Minitest::Test
  include WorkflowTestHelper

  FILE = "forge-stable.yaml"
  PATH = ".github/workflows/#{FILE}"
  CHECKOUT_PIN = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
  SETUP_PYTHON_PIN = "actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065"
  SETUP_CRANE_PIN = "imjasonh/setup-crane@00c9e93efa4e1138c9a7a5c594acd6c75a2fbf0c"
  DISPATCH_ONLY = "${{ github.event_name == 'workflow_dispatch' }}"
  PR_ONLY = "${{ github.event_name == 'pull_request_target' }}"
  BASE_SHA = "${{ github.event.pull_request.base.sha }}"
  DISPATCH_PERMISSIONS = {
    "issues" => "write", "pull-requests" => "write", "contents" => "read", "id-token" => "write",
  }.freeze
  PR_SOURCE = {
    "source_repository" => "${{ github.event.pull_request.head.repo.full_name }}",
    "source_sha" => "${{ github.event.pull_request.head.sha }}",
  }.freeze
  INSTALL = "python -m pip install --disable-pip-version-check click==8.3.3 psutil==5.9.8"
  LOOKUP_STEP = "Run the PR image lookup against trusted history"

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

  def test_pr_runs_use_pull_request_target_with_the_same_paths
    assert_equal %w[pull_request_target workflow_dispatch], trigger(workflow).keys.sort
    assert_equal(
      {"paths" => [".github/workflows/forge-stable.yaml", "testsuite/find_latest_image.py"]},
      trigger(workflow).fetch("pull_request_target"),
    )
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

  # The guard still treats determine-test-metadata and run as PR-controlled,
  # because they use inputs.GIT_SHA as an execution value. The dispatch-only
  # condition removes the pull_request_target source, so a PR event never
  # starts them.
  def test_credentialed_jobs_run_only_on_dispatch
    %w[generate-matrix determine-test-metadata run].each do |name|
      assert_equal DISPATCH_ONLY, job(name).fetch("if"), name
      refute_includes analysis.jobs.fetch(name).sources.map(&:kind), :pull_request_target_workflow, name
    end
    assert_equal DISPATCH_PERMISSIONS, job("determine-test-metadata").fetch("permissions")
    assert_equal DISPATCH_PERMISSIONS, job("run").fetch("permissions")
    metadata_env = job("determine-test-metadata").fetch("env")
    assert_equal "${{ secrets.AWS_ACCESS_KEY_ID }}", metadata_env.fetch("AWS_ACCESS_KEY_ID")
    assert_equal "${{ inputs.IMAGE_TAG }}", metadata_env.fetch("IMAGE_TAG")
  end

  def test_live_lookup_job_is_gated_by_the_fixed_environment
    live = job("pr-lookup-live")
    assert_equal PR_ONLY, live.fetch("if")
    refute live.key?("needs")
    assert_equal 15, live.fetch("timeout-minutes")
    assert_equal "privileged-pr-ci", live.fetch("environment")
    assert_equal({"contents" => "read", "id-token" => "write"}, live.fetch("permissions"))
    refute live.key?("secrets")
    refute live.key?("env")
    %w[AWS_ GIT_CREDENTIALS].each { |token| refute_includes live.to_s, token }

    analyzed = analysis.jobs.fetch("pr-lookup-live")
    assert analyzed.fixed_environment
    assert_empty analyzed.effective_privileges.to_a
  end

  def test_live_lookup_loads_only_trusted_actions_before_pr_code
    live = job("pr-lookup-live")
    live_steps = steps(live)
    lookup = named_step("pr-lookup-live", LOOKUP_STEP)
    # The lookup runs inside trusted-base/ and can rewrite it, so it must be last.
    assert_equal live_steps.length - 1, live_steps.index(lookup)

    trusted = checkout_steps(live).first
    assert_equal CHECKOUT_PIN, trusted.fetch("uses")
    assert_equal(
      {"ref" => BASE_SHA, "path" => "trusted-base", "fetch-depth" => 0, "persist-credentials" => false},
      trusted.fetch("with"),
    )
    source = exact_source_step(live)
    assert_equal "./trusted-base/.github/actions/checkout-exact-pr-source", source.fetch("uses")
    assert_equal PR_SOURCE, source.fetch("with")
    assert_operator live_steps.index(trusted), :<, live_steps.index(source)

    refute live_steps.any? { |step| step["uses"].to_s.start_with?("./pr-source/") }
    live_steps.select { |step| step["uses"].to_s.start_with?("./") }.each do |step|
      assert step.fetch("uses").start_with?("./trusted-base/"), step["name"]
    end
    # An unpinned version installs whatever crane release is newest, with no checksum.
    crane = live_steps.find { |step| step["uses"] == SETUP_CRANE_PIN }
    assert_equal({"version" => "v0.15.2"}, crane.fetch("with"))
    assert_equal({"python-version" => "3.10"}, live_steps.find { |step| step["uses"] == SETUP_PYTHON_PIN }.fetch("with"))
    assert_includes live_steps.filter_map { |step| step["run"]&.strip }, INSTALL

    auth = live_steps.find { |step| step["uses"] == "./trusted-base/.github/actions/gcp-registry-auth" }
    assert_equal(
      {
        "workload_identity_provider" => "${{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}",
        "service_account" => "${{ secrets.GCP_SERVICE_ACCOUNT_EMAIL }}",
        "access_token_lifetime" => 900,
        "create_credentials_file" => false,
      },
      auth.fetch("with"),
    )

    assert_equal "trusted-base", lookup.fetch("working-directory")
    assert_equal "python ../pr-source/testsuite/find_latest_image.py --variant failpoints", lookup.fetch("run").strip
  end

  def test_pr_job_shell_commands_read_values_only_from_env
    %w[pr-lookup-live].each do |name|
      steps(job(name)).filter_map { |step| step["run"] }.each { |script| refute_includes script, "${{", name }
    end
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
end
