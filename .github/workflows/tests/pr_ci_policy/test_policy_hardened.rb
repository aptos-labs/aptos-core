# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"

class PolicyHardenedTest < Minitest::Test
  include PolicyTestHelper

  def test_strict_hardened_workflow_rejects_changes_to_an_existing_risky_job
    base = fixture("legacy-unsafe.yaml")
    head = base.sub("./legacy.sh", "./legacy-v2.sh")
    path = ".github/workflows/mono-move-e2e-perf.yaml"
    assert_includes PrCiPolicy::PolicyChecker.new.check_pair(path, base, head).join("\n"), "hardened workflow"
  end

  def test_strict_hardened_workflow_rejects_changes_to_a_fixed_environment_risky_job
    base = fixture("fixed-environment-privileged.yaml")
    head = base.sub("google-github-actions/auth@v2", "google-github-actions/auth@v3")
    path = ".github/workflows/mono-move-e2e-perf.yaml"

    assert_includes PrCiPolicy::PolicyChecker.new.check_pair(path, base, head).join("\n"), "hardened workflow"
  end

  def test_hardened_workflow_reports_each_job_privilege_once
    path = ".github/workflows/mono-move-e2e-perf.yaml"
    base = fixture("legacy-unsafe.yaml")
    heads = {
      "state and job changed" => base.sub("permissions:\n", "env:\n  MODE: head\npermissions:\n").sub("./legacy.sh", "./legacy-v2.sh"),
      "state changed adds a privilege" => base.sub("  contents: write\n", "  contents: write\n  id-token: write\n"),
    }

    heads.each do |description, head|
      result = PrCiPolicy::PolicyChecker.new.check_pair(path, base, head)
      pairs = result.map { |violation| [violation.job, violation.privilege] }

      refute_empty result, description
      assert_equal pairs.uniq, pairs, description
      assert_equal [:hardened_state_changed], result.map(&:category).uniq, description
    end
  end

  def test_base_and_head_jobs_use_the_same_delegation_rule
    callee = PrCiPolicy::WorkflowAnalysis.new("callee", fixture("protected-reusable-callee.yaml"))
    base = fixture("approved-reusable-caller.yaml")
    head = base.sub("    with:\n", "    secrets:\n      TOKEN: ${{ secrets.DEPLOY_TOKEN }}\n    with:\n")

    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, base, head, callee_resolver: ->(_target) { callee })

    assert_equal(
      [["publish", :privilege_increase, :id_token_write], ["publish", :privilege_increase, :secret]],
      findings(result),
    )
  end

  def test_non_hardened_workflow_allows_non_privilege_changes_to_existing_debt
    base = fixture("legacy-unsafe.yaml")
    head = base.sub("./legacy.sh", "./legacy-v2.sh")
    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, base, head)
  end

  def test_non_hardened_workflow_allows_changes_to_a_fixed_environment_risky_job
    base = fixture("fixed-environment-privileged.yaml")
    head = base.sub("google-github-actions/auth@v2", "google-github-actions/auth@v3")

    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, base, head)
  end

  def test_hardened_workflow_allows_changes_that_remove_all_risky_privileges
    base = fixture("fixed-environment-privileged.yaml")
    head = fixture("new-safe-secretless.yaml")
    path = ".github/workflows/mono-move-e2e-perf.yaml"

    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(path, base, head)
  end

  def test_strict_hardened_workflow_rejects_renamed_protected_reusable_job_with_untrusted_base_sha
    base = File.read(File.join(ROOT, ".github/workflows/docker-build-test.yaml"))
    renamed = base.gsub("pr-node-cli-faucet-tests", "pr-node-cli-faucet-tests-renamed")
    head = renamed.sub(/  pr-node-cli-faucet-tests-renamed:\n.*?(?=\n  pr-forge-e2e:)/m) do |job|
      job.sub(
        "BASE_SHA: ${{ github.event.pull_request.base.sha }}",
        "BASE_SHA: ${{ github.event.pull_request.head.sha }}",
      )
    end
    refute_equal base, renamed
    refute_equal renamed, head

    callee = PrCiPolicy::WorkflowAnalysis.new("callee", fixture("protected-reusable-callee.yaml"))
    resolver = ->(_target) { callee }
    result = PrCiPolicy::PolicyChecker.new.check_pair(
      ".github/workflows/docker-build-test.yaml",
      base,
      head,
      callee_resolver: resolver,
    )

    assert_includes result.join("\n"), "hardened workflow"
  end

  def test_strict_hardened_workflow_rejects_workflow_level_execution_state_changes
    base = <<~YAML
      name: hardened-workflow-state
      on: [pull_request_target]
      permissions:
        contents: write
        issues: write
      env:
        MODE: base
      defaults:
        run:
          shell: bash
      cache-mode: read
      jobs:
        legacy:
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@v4
              with:
                ref: ${{ github.event.pull_request.head.sha }}
            - run: ./legacy.sh
    YAML
    heads = {
      "on" => base.sub("on: [pull_request_target]", "on: [pull_request_target, workflow_dispatch]"),
      "permissions" => base.sub("  issues: write\n", ""),
      "env" => base.sub("  MODE: base", "  MODE: head"),
      "defaults" => base.sub("    shell: bash", "    shell: bash --noprofile -e -o pipefail {0}"),
      "cache-mode" => base.sub("cache-mode: read", "cache-mode: none"),
    }
    path = ".github/workflows/mono-move-e2e-perf.yaml"

    heads.each do |state_key, head|
      refute_equal base, head, state_key
      result = PrCiPolicy::PolicyChecker.new.check_pair(path, base, head)
      assert_includes result.join("\n"), "hardened workflow", state_key
    end
  end

  def test_rejects_policy_workflow_trigger_changes
    base = policy_workflow_text
    head = base.sub(
      "types: [opened, synchronize, reopened]",
      "types: [opened, synchronize, reopened, closed]",
    )

    refute_empty policy_workflow_violations(base, head)
  end

  def test_rejects_policy_workflow_permission_changes
    base = policy_workflow_text
    head = base.sub("  pull-requests: read", "  actions: read")

    refute_empty policy_workflow_violations(base, head)
  end

  def test_rejects_policy_workflow_checkout_changes
    base = policy_workflow_text
    head = base.sub(
      "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683",
      "actions/checkout@22bd71901bbe5b1630ceea73d27597364c9af683",
    )

    refute_empty policy_workflow_violations(base, head)
  end

  def test_rejects_policy_workflow_job_changes
    base = policy_workflow_text
    head = base.sub(
      "name: Validate workflow privilege changes",
      "name: Validate modified workflow privilege changes",
    )

    refute_empty policy_workflow_violations(base, head)
  end

  def test_accepts_identical_and_comment_only_policy_workflow_changes
    base = policy_workflow_text
    comment_only = base.sub("name: PR CI policy", "# Documentation only.\nname: PR CI policy")

    refute_equal base, comment_only
    assert_empty policy_workflow_violations(base, base)
    assert_empty policy_workflow_violations(base, comment_only)
  end

  def test_rejects_removal_of_policy_and_hardened_workflows
    safe = fixture("new-safe-secretless.yaml")
    refute_empty PrCiPolicy::PolicyChecker.new.check_pair(POLICY_WORKFLOW_PATH, safe, nil)
    refute_empty PrCiPolicy::PolicyChecker.new.check_pair(".github/workflows/cli-e2e-tests.yaml", safe, nil)
    refute_empty PrCiPolicy::PolicyChecker.new.check_pair(".github/workflows/docker-build-test-trusted.yaml", safe, nil)
    refute_empty PrCiPolicy::PolicyChecker.new.check_pair(".github/workflows/docker-forge-pr-report.yaml", safe, nil)
    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, safe, nil)
  end

  def test_policy_workflow_uses_only_read_permissions_and_exact_base_checkout
    path = File.join(ROOT, ".github/workflows/pr-ci-policy.yaml")
    workflow = PrCiPolicy::SafeYaml.load(File.read(path), path)
    assert workflow.fetch("on").key?("pull_request_target")
    assert_equal({ "contents" => "read", "pull-requests" => "read" }, workflow.fetch("permissions"))
    job = workflow.fetch("jobs").fetch("policy")
    assert_equal({ "contents" => "read", "pull-requests" => "read" }, job.fetch("permissions"))
    checkout = job.fetch("steps").first
    assert_equal "${{ github.event.pull_request.base.repo.full_name }}", checkout.dig("with", "repository")
    assert_equal "${{ github.event.pull_request.base.sha }}", checkout.dig("with", "ref")
    assert_equal false, checkout.dig("with", "persist-credentials")
  end

  def test_workflow_run_jobs_are_pr_controlled
    default_checkout = { "uses" => "actions/checkout@v4" }
    cases = {
      "checkout of the triggering head SHA" => [
        { "uses" => "actions/checkout@v4", "with" => { "ref" => "${{ github.event.workflow_run.head_sha }}" } },
      ],
      "shell fetch of the triggering head SHA" => [
        default_checkout,
        { "run" => "git fetch origin ${{ github.event.workflow_run.head_sha }} && git checkout FETCH_HEAD && make" },
      ],
      "artifact from the triggering run" => [
        default_checkout,
        {
          "uses" => "actions/download-artifact@v4",
          "with" => { "run-id" => "${{ github.event.workflow_run.id }}", "github-token" => "${{ github.token }}" },
        },
        { "run" => "bash ./artifact/run.sh" },
      ],
      "triggering head branch in a shell command" => [
        { "run" => "echo ${{ github.event.workflow_run.head_branch }}" },
      ],
      "gh run download from the triggering run" => [
        { "run" => "gh run download ${{ github.event.workflow_run.id }} && ./x" },
      ],
    }
    cases.each do |description, steps|
      result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow_run_workflow(steps))

      assert_equal [["follow-up", :new_job, :write_permission]], findings(result), description
      assert_includes result.first.sources.map(&:kind), :workflow_run_workflow, description
    end
  end

  def test_accepts_new_read_only_workflow_run_job
    steps = [{ "run" => "echo ${{ github.event.workflow_run.head_branch }}" }]
    head = workflow_run_workflow(steps, permissions: { "contents" => "read" })

    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, head)
  end

  def test_rejects_pr_artifact_execution_in_hardened_workflow_run_report
    path = ".github/workflows/pr-ci-report.yaml"
    base = File.read(File.join(ROOT, path))
    anchor = "      - name: Validate originating run and render typed report\n"
    run_artifact = "      - name: Run the PR artifact\n" \
                   "        run: gh run download ${{ github.event.workflow_run.id }} -n pr-ci-report-v1 && bash ./run.sh\n"
    assert_includes base, anchor

    result = PrCiPolicy::PolicyChecker.new.check_pair(path, base, base.sub(anchor, run_artifact + anchor))

    assert_equal(
      [["report", :hardened_job_changed, :write_authority], ["report", :hardened_job_changed, :write_permission]],
      findings(result),
    )
  end

  def test_unchanged_workflow_run_workflows_pass
    %w[pr-ci-report docker-forge-pr-report calibrate-execution-performance].each do |name|
      path = ".github/workflows/#{name}.yaml"
      text = File.read(File.join(ROOT, path))

      assert_empty PrCiPolicy::PolicyChecker.new.check_pair(path, text, text), path
    end
  end
end
