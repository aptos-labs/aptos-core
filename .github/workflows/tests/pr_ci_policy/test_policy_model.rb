# frozen_string_literal: true

require "minitest/autorun"
require "set"
require "tmpdir"
require_relative "policy_test_helper"

class PolicyModelTest < Minitest::Test
  include PolicyTestHelper

  def test_manifest_lists_existing_workflows_directories_and_exact_files
    manifest = PrCiPolicy::Manifest.read

    (manifest.hardened_workflows + manifest.approved_protected_reusables).each do |path|
      assert File.file?(File.join(ROOT, path)), path
    end
    manifest.protected_runtime_prefixes.each do |prefix|
      if prefix.end_with?("/")
        assert File.directory?(File.join(ROOT, prefix)), prefix
      else
        assert File.file?(File.join(ROOT, prefix)), prefix
      end
    end
    assert_includes manifest.protected_runtime_prefixes, ".github/ci/"
    assert_includes manifest.protected_runtime_prefixes, ".github/actions/pr-ci-policy/"
    assert_includes manifest.protected_runtime_prefixes, ".github/actions/checkout-exact-pr-source/"
    assert_includes manifest.protected_runtime_prefixes, ".github/actions/gcp-registry-auth/"
    assert_includes manifest.protected_runtime_prefixes, "docker/builder/image-tag-prefix.sh"
    assert_includes manifest.hardened_workflows, POLICY_WORKFLOW_PATH
  end

  def test_exact_protected_runtime_file_does_not_protect_similarly_named_files
    manifest = PrCiPolicy::Manifest.read
    assert manifest.protected_runtime?("docker/builder/image-tag-prefix.sh")
    refute manifest.protected_runtime?("docker/builder/image-tag-prefix.sh.bak")
    refute manifest.protected_runtime?("docker/builder/image-tag-prefix.sh/child")
  end

  def test_checker_reads_hardened_and_approved_paths_from_the_manifest
    manifest = PrCiPolicy::Manifest.new(
      hardened_workflows: Set[PATH],
      approved_protected_reusables: Set[],
      protected_runtime_prefixes: Set[".github/ci/"],
    )
    checker = PrCiPolicy::PolicyChecker.new(manifest: manifest)
    base = fixture("legacy-unsafe.yaml")
    callee = PrCiPolicy::WorkflowAnalysis.new("callee", fixture("protected-reusable-callee.yaml"))

    assert_includes checker.check_pair(PATH, base, base.sub("./legacy.sh", "./legacy-v2.sh")).join("\n"), "hardened workflow"
    refute_empty checker.check_pair(PATH, base, nil)
    refute_empty checker.check_pair(
      ".github/workflows/caller.yaml",
      nil,
      fixture("approved-reusable-caller.yaml"),
      callee_resolver: ->(_target) { callee },
    )
  end

  def test_manifest_read_fails_closed_on_malformed_content
    valid = JSON.parse(File.read(PrCiPolicy::Manifest::PATH))
    cases = {
      "invalid JSON" => "{",
      "missing key" => JSON.generate(valid.except("hardened_workflows")),
      "unknown key" => JSON.generate(valid.merge("extra" => [])),
      "empty list" => JSON.generate(valid.merge("hardened_workflows" => [])),
      "non-string entry" => JSON.generate(valid.merge("hardened_workflows" => [1])),
      "duplicate entry" => JSON.generate(valid.merge("hardened_workflows" => [PATH, PATH])),
      "reusable written as a uses value" => JSON.generate(valid.merge("approved_protected_reusables" => ["./#{PATH}"])),
      "prefix without trailing slash" => JSON.generate(valid.merge("protected_runtime_prefixes" => [".github/ci"])),
    }

    Dir.mktmpdir do |dir|
      path = File.join(dir, "pr-ci-policy.json")
      cases.each do |name, text|
        File.write(path, text)
        assert_raises(PrCiPolicy::PolicyError, name) { PrCiPolicy::Manifest.read(path) }
      end
    end
  end

  def test_safe_yaml_rejects_mapping_keys_that_do_not_load_as_strings
    {
      "root integer" => "1: x\non: push\njobs: {}\n",
      "nested integer" => "on: push\njobs:\n  a:\n    env:\n      1: x\n",
      "nested null" => "on: push\njobs:\n  a:\n    env:\n      ~: x\n",
      "nested float" => "on: push\njobs:\n  a:\n    env:\n      1.5: x\n",
      "tagged integer" => "on: push\njobs:\n  a:\n    env:\n      !!int \"3\": x\n",
      "nested boolean" => "on: push\njobs:\n  a:\n    env:\n      True: x\n",
      "nested on" => "on: push\njobs:\n  a:\n    on: x\n",
      "symbol" => "on: push\njobs:\n  a:\n    env:\n      :name: x\n",
    }.each do |name, text|
      assert_raises(PrCiPolicy::PolicyError, name) { PrCiPolicy::SafeYaml.load(text, name) }
    end
  end

  def test_safe_yaml_accepts_string_keys_and_the_root_on_key
    text = "on: push\njobs:\n  a:\n    env:\n      \"3\": x\n      !!str 4: y\n"

    assert_equal(
      { "on" => "push", "jobs" => { "a" => { "env" => { "3" => "x", "4" => "y" } } } },
      PrCiPolicy::SafeYaml.load(text, "string keys"),
    )
  end

  def test_violations_are_typed_and_render_the_existing_messages
    result = violations(nil, "new-unsafe-oidc-cloud.yaml")

    assert_equal(
      [["test", :new_job, :cloud_auth], ["test", :new_job, :id_token_write], ["test", :new_job, :secret_manager]],
      result.map { |violation| [violation.job, violation.category, violation.privilege.kind] },
    )
    assert_equal(
      %i[checkout_ref execution_value pull_request_target_workflow],
      result.first.sources.map(&:kind).sort,
    )
    assert_equal(
      "#{PATH}: job \"test\": new PR-controlled job combines PR-controlled checkout ref, " \
      "PR-controlled execution value, pull_request_target workflow with id-token: write",
      result[1].to_s,
    )
  end

  def test_risk_kinds_belong_to_exactly_one_dimension
    assert_empty PrCiPolicy::Risk::SOURCE_LABELS.keys & PrCiPolicy::Risk::PRIVILEGE_LABELS.keys
    assert PrCiPolicy::Risk.new(kind: :checkout_ref).source?
    refute PrCiPolicy::Risk.new(kind: :cloud_auth).source?
    assert_equal "write permission (contents: write)",
                 PrCiPolicy::Risk.new(kind: :write_permission, detail: "contents: write").to_s
    assert_raises(ArgumentError) { PrCiPolicy::Risk.new(kind: :checkout_reff) }
  end

  def test_job_analysis_partitions_risks_by_dimension
    job = PrCiPolicy::WorkflowAnalysis.new(PATH, fixture("new-unsafe-oidc-cloud.yaml")).jobs.fetch("test")

    assert_equal %i[checkout_ref execution_value pull_request_target_workflow], job.sources.map(&:kind).sort
    assert_equal %i[cloud_auth id_token_write secret_manager], job.effective_privileges.map(&:kind).sort
    refute job.privileges.any?(&:source?)
  end

  def test_rejects_malformed_yaml_and_aliases
    assert_raises(PrCiPolicy::PolicyError) { violations(nil, "malformed.yaml") }
    assert_raises(PrCiPolicy::PolicyError) { violations(nil, "alias.yaml") }
    assert_raises(PrCiPolicy::PolicyError) { violations(nil, "boolean-key-collision.yaml") }
    assert_raises(PrCiPolicy::PolicyError) { violations(nil, "invalid-step-construct.yaml") }
  end
end
