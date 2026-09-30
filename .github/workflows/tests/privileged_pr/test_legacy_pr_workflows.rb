# frozen_string_literal: true

require "minitest/autorun"
require_relative "../workflow_test_helper"

# Faucet prod and Rust client tests run the exact PR head on pull_request with a
# read-only token and no credentials. The CICD:non-required-tests label only
# limits cost. It is not a trust boundary.
module SecretlessLabelGatedWorkflowTests
  include WorkflowTestHelper

  CHECKOUT_PIN = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
  EXACT_SHA = "${{ github.event.pull_request.head.sha || github.sha }}"
  LABEL_GATE = "contains(github.event.pull_request.labels.*.name, 'CICD:non-required-tests')"
  RUNNER = "runs-on,cpu=64,family=c7,disk=large,image=aptos-ubuntu-x64,run-id=${{ github.run_id }}"

  def path
    ".github/workflows/#{self.class::FILE}"
  end

  def workflow
    @workflow ||= load_workflow(self.class::FILE)
  end

  def test_runs_on_pull_request_and_main_push_only
    assert_equal(
      {
        "pull_request" => {"types" => %w[labeled opened synchronize reopened auto_merge_enabled]},
        "push" => {"branches" => ["main"]},
      },
      trigger(workflow),
    )
  end

  def test_jobs_are_secretless
    assert_equal({"contents" => "read"}, workflow.fetch("permissions"))
    assert_equal self.class::NETWORKS.map { |network| "run-tests-#{network}" }, jobs(workflow).keys
    text = File.read(File.join(ROOT, path))
    %w[secrets. id-token docker-setup permission-check].each { |word| refute_includes text, word }
    analysis = PrCiPolicy::WorkflowAnalysis.new(path, text)
    jobs(workflow).each do |name, job|
      refute job.key?("permissions"), name
      refute job.key?("environment"), name
      refute job.key?("needs"), name
      assert_empty analysis.jobs.fetch(name).privileges.to_a, name
    end
  end

  def test_each_job_tests_the_exact_pr_head_against_public_images
    self.class::NETWORKS.each do |network|
      job = jobs(workflow).fetch("run-tests-#{network}")
      assert_equal LABEL_GATE, job.fetch("if")
      assert_equal RUNNER, job.fetch("runs-on")
      assert_equal 2, steps(job).length
      checkout, tests = steps(job)
      assert_equal CHECKOUT_PIN, checkout.fetch("uses")
      assert_equal({"ref" => EXACT_SHA, "persist-credentials" => false}, checkout.fetch("with"))
      assert_equal "./.github/actions/#{self.class::ACTION}", tests.fetch("uses")
      assert_equal({"NETWORK" => network, "GCP_DOCKER_ARTIFACT_REPO" => "aptoslabs"}, tests.fetch("with"))
    end
  end

  # Hardening makes the guard reject added secrets or OIDC and adds the file
  # to the actionlint set.
  def test_workflow_is_hardened
    assert_includes policy_manifest.fetch("hardened_workflows"), path
  end
end

class FaucetTestsProdWorkflowTests < Minitest::Test
  include SecretlessLabelGatedWorkflowTests

  FILE = "faucet-tests-prod.yaml"
  ACTION = "run-faucet-tests"
  NETWORKS = %w[devnet testnet].freeze

  def test_runs_on_pull_request_and_main_push_only
    assert_equal(
      {
        "workflow_call" => {"inputs" => {"GIT_SHA" => {"required" => true, "type" => "string"}}},
        "pull_request" => {"types" => %w[labeled opened synchronize reopened auto_merge_enabled]},
        "push" => {"branches" => ["main"]},
      },
      trigger(workflow),
    )
  end

  def test_each_job_tests_the_exact_pr_head_against_public_images
    gate = "github.event_name == 'workflow_dispatch' || #{LABEL_GATE}"
    ref = "${{ inputs.GIT_SHA || github.event.pull_request.head.sha || github.sha }}"
    NETWORKS.each do |network|
      job = jobs(workflow).fetch("run-tests-#{network}")
      assert_equal gate, job.fetch("if")
      assert_equal RUNNER, job.fetch("runs-on")
      assert_equal 2, steps(job).length
      checkout, tests = steps(job)
      assert_equal CHECKOUT_PIN, checkout.fetch("uses")
      assert_equal({"ref" => ref, "persist-credentials" => false}, checkout.fetch("with"))
      assert_equal "./.github/actions/#{ACTION}", tests.fetch("uses")
      assert_equal({"NETWORK" => network, "GCP_DOCKER_ARTIFACT_REPO" => "aptoslabs"}, tests.fetch("with"))
    end
  end
end

class RustClientTestsWorkflowTests < Minitest::Test
  include SecretlessLabelGatedWorkflowTests

  FILE = "rust-client-tests.yaml"
  ACTION = "run-rust-client-tests"
  NETWORKS = %w[devnet testnet mainnet].freeze
end

# Backport runs after merge on pull_request_target and never loads PR code. It
# holds APTOS_BOT_PAT without a protected environment, so it is not hardened:
# the guard would reject the secret. The sender check runs in a secretless gate
# job, so the guard's ratchet rule accepts the secret-holding backport job. This
# contract test holds its structure.
class BackportWorkflowTests < Minitest::Test
  include WorkflowTestHelper

  FILE = "backport-to-release-branches.yaml"
  PATH = ".github/workflows/#{FILE}"
  BACKPORT_PIN = "sorenlouv/backport-github-action@ad888e978060bc1b2798690dd9d03c4036560947"
  MERGED_RELEASE_LABEL =
    "github.event.pull_request.merged == true && contains(join(github.event.pull_request.labels.*.name, ','), 'v1.')"
  CHECK_NAME = "Require write permission for the sender"
  GATE_JOB = "authorize-sender"

  def workflow
    @workflow ||= load_workflow(FILE)
  end

  def text
    File.read(File.join(ROOT, PATH))
  end

  def job
    jobs(workflow).fetch("backport")
  end

  def gate
    jobs(workflow).fetch(GATE_JOB)
  end

  def test_runs_base_code_after_merge_with_an_empty_token
    assert_equal({"pull_request_target" => {"types" => %w[labeled closed]}}, trigger(workflow))
    assert_equal({}, workflow.fetch("permissions"))
    assert_equal [GATE_JOB, "backport"], jobs(workflow).keys
    assert_equal MERGED_RELEASE_LABEL, gate.fetch("if")
    refute job.key?("if")
    assert_equal [GATE_JOB], job.fetch("needs")
    [gate, job].each { |j| refute j.key?("permissions"), j.inspect }
    [gate, job].each do |j|
      assert_empty checkout_steps(j)
      steps(j).each { |step| refute step["uses"].to_s.start_with?("./"), step.inspect }
      steps(j).filter_map { |step| step["run"] }.each { |script| refute_includes script, "${{" }
    end
    refute_includes text, "sushichop/"
  end

  def test_gate_job_checks_the_sender_permission
    assert_equal 1, steps(gate).length
    check = steps(gate).first
    assert_equal CHECK_NAME, check.fetch("name")
    assert_equal(
      {
        "GH_TOKEN" => "${{ github.token }}",
        "REPO" => "${{ github.repository }}",
        "SENDER" => "${{ github.event.sender.login }}",
      },
      check.fetch("env"),
    )
    script = check.fetch("run")
    assert_includes script, "set -euo pipefail"
    assert_includes script, %q{gh api "repos/${REPO}/collaborators/${SENDER}/permission" --jq .permission}
    assert_includes script, "admin|write)"
    assert_match(/^\s*\*\).*exit 1/, script)
    refute_includes gate.to_s, "secrets."
    assert_equal BACKPORT_PIN, steps(job).first.fetch("uses")
  end

  def test_pat_reaches_only_the_pinned_backport_action
    assert_equal 1, text.scan("APTOS_BOT_PAT").length
    backport = steps(job).find { |step| step["uses"] == BACKPORT_PIN }
    assert_equal "${{ secrets.APTOS_BOT_PAT }}", backport.fetch("with").fetch("github_token")
  end

  def test_workflow_is_not_hardened
    refute_includes policy_manifest.fetch("hardened_workflows"), PATH
  end
end
