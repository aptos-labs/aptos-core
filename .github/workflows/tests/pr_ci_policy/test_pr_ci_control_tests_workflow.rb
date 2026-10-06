# frozen_string_literal: true

require "minitest/autorun"
require "fileutils"
require "open3"
require "tmpdir"
require_relative "../../../actions/pr-ci-policy/lib/pr_ci_policy"

class PrCiControlTestsWorkflowTest < Minitest::Test
  ROOT = File.expand_path("../../../..", __dir__)
  WORKFLOW_PATH = File.join(ROOT, ".github/workflows/pr-ci-control-tests.yaml")
  COMMIT_PIN = /@[0-9a-f]{40}\z/

  def setup
    @workflow = PrCiPolicy::SafeYaml.load(File.read(WORKFLOW_PATH), WORKFLOW_PATH)
    @jobs = @workflow.fetch("jobs")
  end

  def scripts(job)
    @jobs.fetch(job).fetch("steps").filter_map { |step| step["run"] }.join("\n")
  end

  def test_runs_contract_suites_for_pull_requests_and_main_pushes
    triggers = @workflow.fetch("on")

    assert triggers.key?("pull_request")
    assert_equal ["main"], triggers.fetch("push").fetch("branches")
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

  def test_actions_are_commit_pinned_and_checkouts_do_not_persist_credentials
    @jobs.each do |name, job|
      uses = job.fetch("steps").filter_map { |step| step["uses"] }
      uses.each { |action| assert_match COMMIT_PIN, action, name }
      checkout = job.fetch("steps").find { |step| step.fetch("uses", "").start_with?("actions/checkout@") }

      refute_nil checkout, name
      assert_equal false, checkout.fetch("with").fetch("persist-credentials"), name
    end
  end

  def test_actionlint_is_commit_pinned_and_lints_the_manifest_hardened_workflows
    install = scripts("actionlint").lines.find { |line| line.start_with?("go install ") }

    assert_match %r{\Ago install github\.com/rhysd/actionlint/cmd/actionlint@[0-9a-f]{40}(\s|\z)}, install
    assert_includes scripts("actionlint"), %q{JSON.parse(File.read(".github/ci/pr-ci-policy.json")).fetch("hardened_workflows")}
  end

  def test_semgrep_uses_a_digest_pinned_container_and_requires_the_semgrep_rule_test
    image = @jobs.fetch("semgrep").fetch("container").fetch("image")

    assert_match(/@sha256:[0-9a-f]{64}\z/, image)
    assert_includes scripts("semgrep"), "pull-request-target-code-checkout.yaml"
    assert_includes scripts("semgrep"), "permission-check-bypass.yaml"
  end

  def test_python_contracts_install_only_hashed_wheels
    assert_includes scripts("python_contracts"), "--require-hashes --only-binary=:all:"
  end

  def test_ruby_contracts_fail_when_any_discovered_test_fails_or_none_are_found
    script = @jobs.fetch("ruby_contracts").fetch("steps").find { |step| step["name"] == "Run Ruby contract tests" }.fetch("run")
    {
      "all pass" => [{ "test_a.rb" => 0, "nested/test_b.rb" => 0 }, true],
      "earlier test fails" => [{ "test_a.rb" => 0, "nested/test_b.rb" => 1 }, false],
      "no tests" => [{}, false],
    }.each do |label, (tests, success)|
      Dir.mktmpdir do |dir|
        tests_dir = File.join(dir, ".github/workflows/tests")
        FileUtils.mkdir_p(tests_dir)
        tests.each do |path, code|
          FileUtils.mkdir_p(File.dirname(File.join(tests_dir, path)))
          File.write(File.join(tests_dir, path), code.to_s)
        end
        bin = File.join(dir, "bin")
        FileUtils.mkdir_p(bin)
        File.write(File.join(bin, "bundle"), "#!/bin/bash\necho \"${@: -1}\" >> \"$RUN_LOG\"\nexit \"$(cat \"${@: -1}\")\"\n")
        File.chmod(0o755, File.join(bin, "bundle"))
        log = File.join(dir, "run.log")
        env = { "PATH" => "#{bin}:#{ENV.fetch('PATH')}", "RUN_LOG" => log }

        _, status = Open3.capture2e(env, "bash", "-e", "-c", script, chdir: dir)

        assert_equal success, status.success?, label
        ran = File.exist?(log) ? File.readlines(log, chomp: true).sort : []
        assert_equal tests.keys.map { |path| ".github/workflows/tests/#{path}" }.sort, ran, label
      end
    end
  end

  def test_ruby_contracts_use_locked_dependencies
    job = @jobs.fetch("ruby_contracts")
    setup = job.fetch("steps").find { |step| step["uses"].to_s.start_with?("ruby/setup-ruby@") }

    assert_equal "true", job.fetch("env").fetch("BUNDLE_FROZEN")
    assert_equal false, setup.fetch("with").fetch("bundler-cache")
    assert_includes File.read(File.join(ROOT, ".github/workflows/tests/Gemfile.lock")), "CHECKSUMS\n"
  end
end
