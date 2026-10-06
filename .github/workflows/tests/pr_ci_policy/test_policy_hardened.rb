# frozen_string_literal: true

require "minitest/autorun"
require_relative "policy_test_helper"
require_relative "property_support"

class PolicyHardenedTest < Minitest::Test
  include PolicyTestHelper
  include PolicyPropertySupport

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

    heads.each do |state_key, head|
      refute_equal base, head, state_key
      result = check_hardened(base, head)
      assert_equal [:hardened_state_changed], result.map(&:category).uniq, state_key
    end
  end

  def test_rejects_policy_workflow_execution_changes
    base = policy_workflow_text
    head = "#{base}env:\n  INJECTED: \"1\"\n"

    assert_equal [:policy_workflow_changed], policy_workflow_violations(base, head).map(&:category)
  end

  def test_accepts_comment_only_policy_workflow_changes
    base = policy_workflow_text

    assert_empty policy_workflow_violations(base, "# Documentation only.\n#{base}")
  end

  def test_rejects_removal_of_the_policy_workflow
    safe = fixture("new-safe-secretless.yaml")

    assert_equal [:removed], policy_workflow_violations(safe, nil).map(&:category)
    assert_empty PrCiPolicy::PolicyChecker.new.check_pair(PATH, safe, nil)
  end

  def test_policy_workflow_uses_only_read_permissions_and_exact_base_checkout
    workflow = PrCiPolicy::SafeYaml.load(policy_workflow_text, POLICY_WORKFLOW_PATH)
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
    steps = [
      { "uses" => "actions/download-artifact@v4", "with" => { "run-id" => "${{ github.event.workflow_run.id }}", "github-token" => "${{ github.token }}" } },
      { "run" => "bash ./artifact/run.sh" },
    ]
    result = PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, workflow_run_workflow(steps))

    assert_equal [["follow-up", :new_job, :write_permission]], findings(result)
    assert_includes result.first.sources.map(&:kind), :workflow_run_workflow
  end

  def test_hardened_workflow_run_report_job_is_a_privileged_anchor
    path = ".github/workflows/pr-ci-report.yaml"
    base = File.read(File.join(ROOT, path))
    head = PrCiPolicy::SafeYaml.load(base, path)
    head.fetch("jobs").fetch("report").fetch("steps") << { "run" => "gh run download ${{ github.event.workflow_run.id }} && bash ./run.sh" }

    result = PrCiPolicy::PolicyChecker.new.check_pair(path, base, JSON.pretty_generate(head))

    assert_equal [["report", :hardened_job_changed]], result.map { |violation| [violation.job, violation.category] }.uniq
  end

  HARDENED_PATH = ".github/workflows/mono-move-e2e-perf.yaml"
  DOCKER_PATH = ".github/workflows/docker-build-test.yaml"

  # Test-owned transition categories and reported privilege sets.
  TRANSITIONS = [
    [:unchanged, nil, nil, [:write_permission]],
    [:job_changed, :hardened_job_changed, nil, [:write_permission]],
    [:state_changed, :hardened_state_changed, nil, [:write_permission]],
    [:state_and_job_changed, :hardened_state_changed, nil, [:write_permission]],
    [:removed_privilege, nil, nil, []],
    [:gained_privilege, :hardened_state_changed, :privilege_increase, [:id_token_write, :write_permission]],
    [:renamed, :hardened_job_changed, :new_job, [:write_permission]],
    [:source_added, :hardened_job_changed, :new_source, [:write_permission]],
    [:new_job, :hardened_job_changed, :new_job, [:write_permission]],
  ].freeze

  def test_generated_base_head_hardened_and_ratchet_transitions
    corpus = (0...TRANSITIONS.length).to_a.product([0, 1], [0, 1]).map { |row| row + [0, 0] }
    check_property("hardened_ratchet_transitions", corpus: corpus,
      generate: ->(random) { [random.rand(TRANSITIONS.length), random.rand(2), random.rand(2), random.rand(4), random.rand(2)] },
      describe: ->(choices) { transition_case(choices).inspect }) do |choices|
      base, head, path, expected = transition_case(choices)
      assert_equal expected, findings(PrCiPolicy::PolicyChecker.new.check_pair(path, base, head)).sort
    end
  end

  def test_upstream_change_is_reported_for_each_dependent_privilege
    base = needs_workflow("authorize" => plain_job, "publish" => privileged_job(needs: "authorize"))
    head = needs_workflow("authorize" => plain_job(run: "./authorize-v2.sh"), "publish" => privileged_job(needs: "authorize"))
    result = check_hardened(base, head)

    assert_equal [["authorize", :hardened_upstream_changed, "publish", :id_token_write]], upstream_findings(result)
    anchor = PrCiPolicy::WorkflowAnalysis.new(HARDENED_PATH, head).jobs.fetch("publish")
    assert_equal anchor.sources, result.first.sources
  end

  def test_transitive_and_diamond_upstream_changes_name_the_anchor_once
    jobs = ->(run) {
      { "a" => plain_job(run: run), "b" => plain_job(needs: "a"), "c" => plain_job(needs: "a"), "publish" => privileged_job(needs: %w[b c]) }
    }
    result = check_hardened(needs_workflow(jobs.call("./a.sh")), needs_workflow(jobs.call("./a-v2.sh")))

    assert_equal [["a", :hardened_upstream_changed, "publish", :id_token_write]], upstream_findings(result)
  end

  def test_one_upstream_change_reports_every_dependent_anchor
    jobs = ->(run) {
      { "authorize" => plain_job(run: run), "publish-a" => privileged_job(needs: "authorize"), "publish-b" => privileged_job(needs: "authorize") }
    }
    result = check_hardened(needs_workflow(jobs.call("./authorize.sh")), needs_workflow(jobs.call("./authorize-v2.sh")))

    assert_equal [
      ["authorize", :hardened_upstream_changed, "publish-a", :id_token_write],
      ["authorize", :hardened_upstream_changed, "publish-b", :id_token_write],
    ], upstream_findings(result)
  end

  def test_changed_job_without_privileged_dependents_passes
    jobs = ->(run) {
      { "authorize" => plain_job, "local" => plain_job(needs: "authorize", run: run), "publish" => privileged_job(needs: "authorize") }
    }

    assert_empty check_hardened(needs_workflow(jobs.call("./local.sh")), needs_workflow(jobs.call("./local-v2.sh")))
  end

  def test_anchor_that_is_also_upstream_is_reported_only_as_an_anchor
    jobs = ->(run) {
      { "authorize" => plain_job, "publish" => privileged_job(needs: "authorize", run: run), "forge" => privileged_job(needs: %w[authorize publish]) }
    }
    result = check_hardened(needs_workflow(jobs.call("./publish.sh")), needs_workflow(jobs.call("./publish-v2.sh")))

    assert_equal [["publish", :hardened_job_changed, nil, :id_token_write]], upstream_findings(result)
  end

  def test_execution_state_change_reports_upstream_jobs_with_their_dependents
    jobs = { "authorize" => plain_job, "local" => plain_job, "publish" => privileged_job(needs: "authorize") }
    base = needs_workflow(jobs)
    head = JSON.pretty_generate(JSON.parse(base).merge("env" => { "MODE" => "head" }))

    assert_equal [
      ["authorize", :hardened_state_changed, "publish", :id_token_write],
      ["publish", :hardened_state_changed, nil, :id_token_write],
    ], upstream_findings(check_hardened(base, head))
  end

  def test_renamed_upstream_job_and_new_hardened_workflow_count_as_changed
    base = needs_workflow("authorize" => plain_job, "publish" => privileged_job(needs: "authorize"))
    renamed = needs_workflow("authorize-v2" => plain_job, "publish" => privileged_job(needs: "authorize-v2"))
    expected = [
      ["authorize-v2", :hardened_upstream_changed, "publish", :id_token_write],
      ["publish", :hardened_job_changed, nil, :id_token_write],
    ]

    assert_equal expected, upstream_findings(check_hardened(base, renamed))
    assert_equal [
      ["authorize", :hardened_upstream_changed, "publish", :id_token_write],
      ["publish", :hardened_job_changed, nil, :id_token_write],
    ], upstream_findings(check_hardened(nil, base))
  end

  def test_docker_authorization_change_is_reported_for_every_privileged_dependent
    base = File.read(File.join(ROOT, DOCKER_PATH))
    head = PrCiPolicy::SafeYaml.load(base, DOCKER_PATH)
    head.fetch("jobs").fetch("compute-authorization").fetch("steps") << { "run" => "./authorize-v2.sh" }
    head = JSON.pretty_generate(head)
    privileged = PrCiPolicy::WorkflowAnalysis.new(DOCKER_PATH, head).jobs
                                           .select { |_name, job| job.pr_controlled? && !job.privileges.empty? }.keys.sort
    result = PrCiPolicy::PolicyChecker.new.check_pair(DOCKER_PATH, base, head)

    refute_empty privileged
    assert_equal [["compute-authorization", :hardened_upstream_changed]], result.map { |violation| [violation.job, violation.category] }.uniq
    assert_equal privileged, result.map(&:dependent).uniq.sort
  end

  private

  def needs_workflow(jobs)
    JSON.pretty_generate("name" => "needs", "on" => "pull_request_target", "permissions" => { "contents" => "read" }, "jobs" => jobs)
  end

  def plain_job(needs: nil, run: "./authorize.sh")
    job = { "runs-on" => "ubuntu-latest", "permissions" => { "contents" => "read" }, "steps" => [{ "run" => run }] }
    needs.nil? ? job : job.merge("needs" => needs)
  end

  def privileged_job(needs:, run: "./publish.sh")
    plain_job(needs: needs, run: run).merge("permissions" => { "contents" => "read", "id-token" => "write" })
  end

  def check_hardened(base, head)
    PrCiPolicy::PolicyChecker.new.check_pair(HARDENED_PATH, base, head)
  end

  def upstream_findings(result)
    result.map { |violation| [violation.job, violation.category, violation.dependent, violation.privilege.kind] }.sort_by { |row| row.map(&:to_s) }
  end

  def transition_case(choices)
    transition, hardened, protected, count, syntax = choices.zip([TRANSITIONS.length, 2, 2, 4, 2]).map { |value, limit| value % limit }
    operation, strict_category, ratchet_category, kinds = TRANSITIONS.fetch(transition)
    base = JSON.parse(pr_target_workflow(permissions: { "contents" => "write" }, steps: [{ "run" => "./legacy.sh" }]))
    job = base.fetch("jobs").fetch("test")
    job["environment"] = "privileged-pr-ci" if protected == 1
    names = Array.new(count + 1) { |index| "test_#{index}" }
    base["jobs"] = names.to_h { |name| [name, job] }
    head = Marshal.load(Marshal.dump(base))
    case operation
    when :job_changed then head["jobs"].each_value { |value| value["steps"].last["run"] = "./legacy-v2.sh" }
    when :state_changed then head["env"] = { "MODE" => "head" }
    when :state_and_job_changed
      head["env"] = { "MODE" => "head" }
      head["jobs"].each_value { |value| value["steps"].last["run"] = "./legacy-v2.sh" }
    when :removed_privilege then head["permissions"] = { "contents" => "read" }
    when :gained_privilege then head["permissions"]["id-token"] = "write"
    when :renamed then head["jobs"] = head["jobs"].transform_keys { |name| "#{name}_renamed" }
    when :source_added then head["jobs"].each_value { |value| value["steps"] << { "run" => "gh pr checkout 42" } }
    when :new_job then base["jobs"] = {}
    end
    # The fixed environment does not exempt a job from the ratchet.
    category = hardened == 1 ? strict_category : ratchet_category
    reported = hardened.zero? && operation == :gained_privilege ? [:id_token_write] : kinds
    expected = category ? head["jobs"].keys.flat_map { |name| reported.map { |kind| [name, category, kind] } }.sort : []
    path = hardened == 1 ? HARDENED_PATH : PATH
    texts = [base, head].map { |document| syntax.zero? ? JSON.generate(document) : JSON.pretty_generate(document) }
    [*texts, path, expected]
  end
end
