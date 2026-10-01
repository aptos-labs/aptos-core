# frozen_string_literal: true

require "minitest/autorun"
require "set"
require "tmpdir"
require_relative "policy_test_helper"
require_relative "property_support"

class PolicyModelTest < Minitest::Test
  include PolicyTestHelper
  include PolicyPropertySupport

  def test_manifest_lists_existing_workflows_directories_and_exact_files
    manifest = PrCiPolicy::Manifest.read

    paths = manifest.hardened_workflows + manifest.approved_protected_reusables + manifest.protected_runtime_prefixes
    paths.each do |path|
      predicate = path.end_with?("/") ? :directory? : :file?
      assert File.public_send(predicate, File.join(ROOT, path)), path
    end
    %w[.github/ci/ .github/actions/pr-ci-policy/ .github/actions/checkout-exact-pr-source/ .github/actions/gcp-registry-auth/ docker/builder/image-tag-prefix.sh].each do |prefix|
      assert_includes manifest.protected_runtime_prefixes, prefix
    end
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
    keys = %w[hardened_workflows approved_protected_reusables protected_runtime_prefixes]
    shapes = [nil, false, 1, "path", {}, [], [1], :duplicate, :bad_path, :nested, :valid]
    render = lambda do |choice|
      key = keys.fetch(choice[0] % keys.size)
      shape = shapes.fetch(choice[1] % shapes.size)
      path = key == "protected_runtime_prefixes" ? ".github/ci/" : PATH
      value = case shape
              when :duplicate then [path, path]
              when :bad_path then [key == "protected_runtime_prefixes" ? ".github/ci" : "./#{PATH}"]
              when :nested then (2 + choice[2] % 2).times.reduce(path) { |nested, _| [nested] }
              when :valid then (1 + choice[2] % 4).times.map { |index| key == "protected_runtime_prefixes" ? ".github/generated-#{index}/" : ".github/workflows/generated-#{index}.yaml" }
              else shape
              end
      JSON.generate(valid.merge(key => value))
    end
    Dir.mktmpdir do |dir|
      path = File.join(dir, "pr-ci-policy.json")
      { "invalid JSON" => "{", "missing key" => JSON.generate(valid.except("hardened_workflows")),
        "unknown key" => JSON.generate(valid.merge("extra" => [])),
        **[nil, false, 1, "path", []].to_h { |root| ["root #{root.inspect}", JSON.generate(root)] } }.each do |name, text|
        File.write(path, text)
        assert_raises(PrCiPolicy::PolicyError, name) { PrCiPolicy::Manifest.read(path) }
      end
      corpus = keys.each_index.to_a.product(shapes.each_index.to_a, (0...4).to_a)
      check_property("manifest shapes", corpus: corpus,
                     generate: ->(random) { [random.rand(keys.size), random.rand(shapes.size), random.rand(4)] }, describe: render) do |choice|
        text = render.call(choice)
        File.write(path, text)
        if shapes.fetch(choice[1] % shapes.size) == :valid
          manifest = PrCiPolicy::Manifest.read(path)
          expected = JSON.parse(text)
          keys.each { |key| assert_equal expected.fetch(key).to_set, manifest.public_send(key) }
        else
          assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::Manifest.read(path) }
        end
      end
    end
  end

  def test_safe_yaml_key_shapes_at_bounded_nesting
    # Explicit YAML key grammar, independent of the production key loader.
    invalid = ["1", "~", "1.5", '!!int "3"', "True", "on", ":name", "[a, b]", "{a: b}"]
    keys = invalid + ['"3"', "!!str 4", "name"]
    render = lambda do |choice|
      key, depth = keys.fetch(choice[0] % keys.size), choice[1] % 5
      next "on: push\njobs: {}\n" if key == "on" && depth.zero?
      next "on: push\n#{key}: x\njobs: {}\n" if depth.zero?

      parents = (1...depth).map { |level| "#{'  ' * level}#{level == 1 ? 'a' : 'env'}:\n" }.join
      "on: push\njobs:\n#{parents}#{'  ' * depth}#{key}: x\n"
    end
    originals = ["1: x\non: push\njobs: {}\n", *invalid.values_at(0, 1, 2, 3, 4, 6).map { |key| "on: push\njobs:\n  a:\n    env:\n      #{key}: x\n" }, "on: push\njobs:\n  a:\n    on: x\n"]
    originals.each { |text| assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::SafeYaml.load(text, "original invalid key") } }
    text = "on: push\njobs:\n  a:\n    env:\n      \"3\": x\n      !!str 4: y\n"
    assert_equal({ "on" => "push", "jobs" => { "a" => { "env" => { "3" => "x", "4" => "y" } } } }, PrCiPolicy::SafeYaml.load(text, "string keys"))
    corpus = keys.each_index.to_a.product((0...5).to_a)
    check_property("YAML key shapes", corpus: corpus,
                   generate: ->(random) { [random.rand(keys.size), random.rand(5)] }, describe: render) do |choice|
      key = keys.fetch(choice[0] % keys.size)
      if invalid.include?(key) && !(key == "on" && (choice[1] % 5).zero?)
        assert_raises(PrCiPolicy::PolicyError) { PrCiPolicy::SafeYaml.load(render.call(choice), "invalid key") }
      else
        data = PrCiPolicy::SafeYaml.load(render.call(choice), "string key")
        assert_equal "push", data.fetch("on")
        unless key == "on"
          depth = choice[1] % 5
          nested = depth.zero? ? data : data.fetch("jobs")
          (1...depth).each { |level| nested = nested.fetch(level == 1 ? "a" : "env") }
          assert_equal "x", nested.fetch({ '"3"' => "3", "!!str 4" => "4" }.fetch(key, key))
        end
      end
    end
  end

  def test_violations_and_job_analysis_partition_typed_risks
    result = violations(nil, "new-unsafe-oidc-cloud.yaml")
    job = PrCiPolicy::WorkflowAnalysis.new(PATH, fixture("new-unsafe-oidc-cloud.yaml")).jobs.fetch("test")
    assert_equal [["test", :new_job, :cloud_auth], ["test", :new_job, :id_token_write], ["test", :new_job, :secret_manager]], findings(result)
    [result.first.sources, job.sources].each do |sources|
      assert_equal %i[checkout_ref execution_value pull_request_target_workflow], sources.map(&:kind).sort
    end
    assert_equal %i[cloud_auth id_token_write secret_manager], job.effective_privileges.map(&:kind).sort
    refute job.privileges.any?(&:source?)
    assert_equal "#{PATH}: job \"test\": new PR-controlled job combines PR-controlled checkout ref, " \
                 "PR-controlled execution value, pull_request_target workflow with id-token: write", result[1].to_s
  end

  def test_risk_kinds_belong_to_exactly_one_dimension
    assert_empty PrCiPolicy::Risk::SOURCE_LABELS.keys & PrCiPolicy::Risk::PRIVILEGE_LABELS.keys
    assert PrCiPolicy::Risk.new(kind: :checkout_ref).source?
    refute PrCiPolicy::Risk.new(kind: :cloud_auth).source?
    assert_equal "write permission (contents: write)",
                 PrCiPolicy::Risk.new(kind: :write_permission, detail: "contents: write").to_s
    assert_raises(ArgumentError) { PrCiPolicy::Risk.new(kind: :checkout_reff) }
  end

  def test_rejects_malformed_yaml_and_aliases
    {
      "malformed.yaml" => "name: malformed\non: [pull_request_target\njobs: {}\n",
      "alias.yaml" => "name: alias\non: pull_request_target\npermissions: &permissions\n  contents: read\njobs:\n  test:\n    runs-on: ubuntu-latest\n    permissions: *permissions\n    steps: []\n",
      "boolean-key-collision.yaml" => "name: collision\non: pull_request_target\ntrue: workflow_dispatch\npermissions:\n  contents: read\njobs: {}\n",
      "invalid-step-construct.yaml" => "name: invalid-step\non: pull_request_target\npermissions:\n  contents: read\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: false\n",
    }.each do |name, text|
      assert_raises(PrCiPolicy::PolicyError, name) { PrCiPolicy::PolicyChecker.new.check_pair(PATH, nil, text) }
    end
  end
end
