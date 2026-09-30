# frozen_string_literal: true

require "minitest/autorun"
require_relative "../../../actions/pr-ci-policy/lib/pr_ci_policy"

class PrCiControlTestsWorkflowTest < Minitest::Test
  ROOT = File.expand_path("../../../..", __dir__)
  WORKFLOW_PATH = File.join(ROOT, ".github/workflows/pr-ci-control-tests.yaml")
  CHECKOUT_SHA = "11bd71901bbe5b1630ceea73d27597364c9af683"
  SEMGREP_IMAGE = "returntocorp/semgrep:1.136.0@sha256:61a7ab31cdab865212ae8ed3ddefc0a51d61ef3b016a63e79848f56241d4586d"

  def setup
    @workflow = PrCiPolicy::SafeYaml.load(File.read(WORKFLOW_PATH), WORKFLOW_PATH)
    @jobs = @workflow.fetch("jobs")
  end

  def test_runs_contract_suites_for_pull_requests_and_main_pushes
    triggers = @workflow.fetch("on")

    assert triggers.key?("pull_request")
    assert_equal ["main"], triggers.fetch("push").fetch("branches")
    assert_equal %w[actionlint python_contracts ruby_contracts rust_contracts semgrep].sort,
                 @jobs.keys.sort
  end

  def test_all_jobs_are_secretless_github_hosted_read_only_contract_checks
    assert_equal({ "contents" => "read" }, @workflow.fetch("permissions"))

    @jobs.each do |name, job|
      assert_equal "ubuntu-latest", job.fetch("runs-on"), name
      refute job.key?("environment"), name
      refute_includes job.to_s, "secrets.", name
      refute_includes job.to_s, "id-token", name
      refute_includes job.to_s, "actions/cache", name
    end
  end

  def test_source_jobs_use_immutable_checkout_without_persisted_credentials
    @jobs.each do |name, job|
      checkout = job.fetch("steps").find { |step| step.fetch("uses", "").start_with?("actions/checkout@") }

      refute_nil checkout, name
      assert_equal "actions/checkout@#{CHECKOUT_SHA}", checkout.fetch("uses"), name
      assert_equal false, checkout.fetch("with").fetch("persist-credentials"), name
    end
  end

  def test_actionlint_is_pinned_and_lints_the_manifest_hardened_workflows
    steps = @jobs.fetch("actionlint").fetch("steps")
    lint = steps.find { |step| step["name"] == "Lint hardened GitHub Actions workflows" }

    assert_includes steps.filter_map { |step| step["run"] }.join("\n"), "github.com/rhysd/actionlint/cmd/actionlint@v1.7.7"
    assert_equal "bash", lint.fetch("shell")
    assert_includes lint.fetch("run"), %q{JSON.parse(File.read(".github/ci/pr-ci-policy.json")).fetch("hardened_workflows")}
    assert_includes lint.fetch("run"), %q{xargs --no-run-if-empty "$actionlint_bin"}
  end

  def test_semgrep_uses_the_pinned_container_and_requires_the_semgrep_rule_test
    job = @jobs.fetch("semgrep")
    script = job.fetch("steps").map { |step| step["run"] }.compact.join("\n")

    assert_equal SEMGREP_IMAGE, job.fetch("container").fetch("image")
    assert_includes script, "pull-request-target-code-checkout.yaml"
    assert_includes script, "permission-check-bypass.yaml"
  end

  def test_semgrep_job_runs_the_rule_contract_without_a_host_dependency
    job = @jobs.fetch("semgrep")
    script = job.fetch("steps").map { |step| step["run"] }.compact.join("\n")

    assert_includes script, "semgrep --config"
    assert_includes script, "python3 -c"
  end

  def test_rust_contract_job_runs_the_trusted_indexer_helper_fixture_suite
    job = @jobs.fetch("rust_contracts")
    script = job.fetch("steps").map { |step| step["run"] }.compact.join("\n")

    assert_includes script, "cargo test -p aptos-indexer-transaction-generator --test ci_helper"
  end

  def test_python_contracts_install_hashed_test_dependencies_in_an_isolated_environment
    job = @jobs.fetch("python_contracts")
    steps = job.fetch("steps")
    setup = steps.find { |step| step["uses"].to_s.start_with?("actions/setup-python@") }
    assert_equal "actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065", setup.fetch("uses")
    assert_equal "3.12", setup.fetch("with").fetch("python-version")
    install = steps.find { |step| step["name"] == "Install Python test dependencies" }
    run = steps.find { |step| step["name"] == "Run Python contract tests" }
    assert_operator steps.index(install), :<, steps.index(run)
    assert_includes install.fetch("run"), 'python3 -m venv "$RUNNER_TEMP/ci-contract-tests-venv"'
    assert_includes install.fetch("run"), "--require-hashes --only-binary=:all: -r .github/ci/requirements-forge-test.txt"
    assert_includes run.fetch("run"), 'source "$RUNNER_TEMP/ci-contract-tests-venv/bin/activate"'
    assert_equal "ci", run.fetch("env").fetch("HYPOTHESIS_PROFILE")
    assert_equal "python3 .github/ci/run_python_tests.py", run.fetch("run").lines.last.strip
    runner = File.read(File.join(ROOT, ".github/ci/run_python_tests.py"))
    %w[central micro-report e2e-report forge faucet-images e2e-images offline-images].each do |suite|
      assert_includes runner, %Q{"#{suite}":}
    end
  end

  def test_ruby_contracts_discover_every_workflow_test_file
    script = @jobs.fetch("ruby_contracts").fetch("steps").map { |step| step["run"] }.compact.join("\n")

    assert_includes script, "find .github/workflows/tests -type f -name 'test_*.rb'"
    assert_includes script, 'test "${#tests[@]}" -gt 0'
    assert_includes script, 'for test_file in "${tests[@]}"; do ruby "$test_file" || status=1; done'
    assert_includes script, 'exit "$status"'
  end
end
