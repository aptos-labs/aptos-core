# frozen_string_literal: true

require "minitest/autorun"
require "pathname"
require_relative "policy_test_helper"

# Privileged jobs run some repository files from a trusted base checkout. A PR
# must not change those files without an admin bypass of the policy check.
class PolicyRuntimeCoverageTest < Minitest::Test
  include PolicyTestHelper

  # Files that privileged PR jobs run or load from the base checkout. Add a
  # file here, and to the manifest, when a privileged job starts to use it.
  KNOWN_RUNTIME_FILES = %w[
    .dockerignore
    .python-version
    docker/builder/builder.Dockerfile
    docker/builder/docker-bake-rust-all.hcl
    docker/builder/docker-bake-rust-all.sh
    docker/builder/forge.Dockerfile
    docker/builder/image-tag-prefix.sh
    ecosystem/indexer-grpc/indexer-transaction-generator/Cargo.toml
    ecosystem/indexer-grpc/indexer-transaction-generator/src/bin/indexer_ci_helper.rs
    testsuite/__init__.py
    testsuite/determinator.py
    testsuite/determine_target_branch_to_fetch_last_released_image.py
    testsuite/find_latest_image.py
    testsuite/forge-test-runner-template.yaml
    testsuite/forge.env
    testsuite/forge.py
    testsuite/run_forge.sh
    testsuite/test_framework/shell.py
  ].freeze
  KNOWN_RUNTIME_DIRECTORIES = %w[terraform/helm/].freeze
  # PR data that a privileged job reads from the PR checkout, not runtime code.
  PR_DATA_DIRECTORY = "ecosystem/indexer-grpc/indexer-transaction-generator/imported_transactions/"
  PR_REACHABLE_EVENTS = %w[pull_request pull_request_target workflow_run].freeze
  BASE_CHECKOUT = %r{\A(?:trusted-base|\.trusted-ci|trusted-refresh)/}

  def manifest
    @manifest ||= PrCiPolicy::Manifest.read
  end

  def test_manifest_protects_known_privileged_runtime_files
    KNOWN_RUNTIME_FILES.each do |path|
      assert File.file?(File.join(ROOT, path)), "#{path} does not exist"
      assert manifest.protected_runtime?(path), "#{path} is not protected"
    end
    KNOWN_RUNTIME_DIRECTORIES.each do |path|
      assert File.directory?(File.join(ROOT, path)), "#{path} does not exist"
      assert manifest.protected_runtime?("#{path}added-by-pr.txt"), "#{path} is not protected"
    end
    assert File.directory?(File.join(ROOT, PR_DATA_DIRECTORY))
    refute manifest.protected_runtime?("#{PR_DATA_DIRECTORY}imported_transactions.yaml")
  end

  # Upstream jobs are included because they steer privileged jobs through outputs and results.
  def test_pr_reachable_privileged_jobs_and_their_upstream_jobs_use_only_protected_local_code
    checked = 0
    pr_reachable_workflows.each do |path, analysis|
      privileged = analysis.jobs.select { |_, job| !job.privileges.empty? }.keys
      in_scope = privileged.to_set | privileged.flat_map { |name| analysis.upstream_of(name).to_a }
      analysis.jobs.each do |name, job|
        next unless in_scope.include?(name)

        if job.reusable_target.is_a?(String) && job.reusable_target.start_with?("./")
          target = job.reusable_target.delete_prefix("./")
          assert manifest.hardened?(target), "#{path}: job #{name.inspect} calls unhardened local workflow #{target}"
          checked += 1
        end
        Array(job.raw["steps"]).each do |step|
          action = step["uses"]
          next unless action.is_a?(String) && action.start_with?("./")

          local = "#{Pathname.new(action).cleanpath.to_s.sub(BASE_CHECKOUT, "")}/"
          assert manifest.protected_runtime?(local),
                 "#{path}: job #{name.inspect} uses #{action}; add #{local} to protected_runtime_prefixes in .github/ci/pr-ci-policy.json"
          checked += 1
        end
      end
    end
    assert_operator checked, :>, 0
  end

  private

  def pr_reachable_workflows
    manifest.hardened_workflows.sort.filter_map do |path|
      analysis = PrCiPolicy::WorkflowAnalysis.new(path, File.read(File.join(ROOT, path)))
      reachable = analysis.events.intersect?(PR_REACHABLE_EVENTS.to_set) || manifest.approved_protected_reusables.include?(path)
      [path, analysis] if reachable
    end
  end
end
