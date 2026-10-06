# frozen_string_literal: true

require "minitest/autorun"
require "digest"
require "open3"
require "tempfile"
require_relative "../workflow_test_helper"
require_relative "../pr_ci_policy/property_support"

class PrivilegedPrWorkflowTests < Minitest::Test
  include WorkflowTestHelper
  include PolicyPropertySupport

  WORKFLOWS = {
    "workflow-run-docker-rust-publish-pr.yaml" => "publish-images",
    "workflow-run-pr-e2e-tests.yaml" => "prepare-images",
    "workflow-run-forge-pr.yaml" => "forge",
  }.freeze

  def forge_step(name)
    steps(jobs(load_workflow("workflow-run-forge-pr.yaml")).fetch("forge")).find { |step| step["name"] == name }
  end

  # Runs one workflow `run:` block with a scratch GITHUB_ENV and returns the lines it wrote.
  def run_step_env(step, env)
    Tempfile.create("github-env") do |file|
      _stdout, stderr, status = Open3.capture3(
        env.merge("PATH" => ENV.fetch("PATH"), "GITHUB_ENV" => file.path), "bash", "-c", step.fetch("run"),
      )
      assert status.success?, stderr
      File.read(file.path).lines(chomp: true)
    end
  end

  def test_each_privileged_job_has_a_fixed_environment_and_no_ambient_authority
    WORKFLOWS.each do |file, job_name|
      job = jobs(load_workflow(file)).fetch(job_name)
      assert_equal "privileged-pr-ci", job.fetch("environment"), file
      assert_equal({"contents" => "read", "id-token" => "write"}, job.fetch("permissions"), file)
      source = File.read(File.join(ROOT, ".github", "workflows", file))
      %w[GIT_CREDENTIALS cache-from cache-to secrets:\ inherit @main].each do |token|
        refute_includes source, token, file
      end
      refute_includes job.to_s, "pr-source", file
    end
  end

  # The guard rejects PR changes to privileged jobs only in hardened workflows,
  # and actionlint checks only these files. Removing an entry needs an admin bypass.
  def test_reviewed_pr_workflows_stay_hardened
    %w[adhoc-forge docker-build-test docker-build-test-trusted docker-forge-pr-report faucet-tests-prod
       forge-lookup-unit-tests forge-stable module-verify rust-client-tests].each do |name|
      assert_includes policy_manifest.fetch("hardened_workflows"), ".github/workflows/#{name}.yaml"
    end
  end

  def test_image_consumers_never_build_and_download_only_producer_artifact_ids
    %w[workflow-run-pr-e2e-tests.yaml workflow-run-forge-pr.yaml].each do |file|
      job = jobs(load_workflow(file)).values.first
      %w[docker-bake setup-buildx image_tag_prefix privileged-pr-setup].each { |token| refute_includes job.to_s, token, file }
      job_steps = steps(job)
      downloads = job_steps.select { |step| step["uses"].to_s.start_with?("actions/download-artifact@") }
      assert_equal 1, downloads.length
      assert_equal "${{ inputs.IMAGE_MANIFEST_ID }}", downloads.first.dig("with", "artifact-ids")
      refute downloads.first.fetch("with").key?("name")
      id_check = job_steps.find { |step| step.dig("env", "IMAGE_MANIFEST_ID") }
      assert_includes id_check.fetch("run"), "^[1-9][0-9]*$"
      assert_operator job_steps.index(id_check), :<, job_steps.index(downloads.first)
      verify = job_steps.find { |step| step.dig("with", "mode") == "verify" }
      assert_equal "./trusted-base/.github/actions/protected-image-manifest", verify.fetch("uses")
      assert_equal "${{ inputs.SOURCE_SHA }}", verify.dig("with", "source_sha")
      assert_equal "${{ github.run_id }}", verify.dig("with", "run_id")
      assert_operator job_steps.index(downloads.first), :<, job_steps.index(verify)
    end
  end

  def test_pr_workflow_call_interfaces_do_not_accept_repository_secrets
    WORKFLOWS.each_key do |file|
      call = trigger(load_workflow(file)).fetch("workflow_call")
      refute call.key?("secrets"), file
      %w[SOURCE_REPOSITORY SOURCE_SHA PR_NUMBER BASE_SHA].each do |input|
        assert_equal true, call.fetch("inputs").fetch(input).fetch("required"), "#{file}: #{input}"
      end
    end
  end

  def test_forge_resolves_baseline_with_trusted_base_owned_mapping
    target = forge_step("Determine the trusted baseline branch")
    assert_equal "trusted-base/testsuite", target.fetch("working-directory")
    assert_includes target.fetch("run"), "determine_target_branch_to_fetch_last_released_image.py"
    refute_includes target.fetch("run"), "pr-source"

    baseline = forge_step("Resolve a trusted baseline without executing branch code")
    assert_equal "${{ steps.baseline-target.outputs.TARGET_BRANCH }}", baseline.fetch("env").fetch("TARGET_BRANCH")
  end

  def test_forge_namespace_derivation_is_deterministic_sha_scoped_and_bounded
    step = forge_step("Derive a bounded PR-and-SHA-scoped namespace")
    derive = lambda do |input|
      lines = run_step_env(step, input)
      lines.grep(/\AFORGE_NAMESPACE=/).first.delete_prefix("FORGE_NAMESPACE=")
    end
    original = [9_999_999_998, 14, *(0...15), *Array.new(40, 10)]
    check_property("forge_namespace", corpus: [original, original.take(17) + Array.new(40, 11)],
      generate: ->(rng) {
        [rng.rand(0...9_999_999_999), rng.rand(0...15),
         *Array.new(15) { rng.rand(0...37) }, *Array.new(40) { rng.rand(0...16) }]
      },
      describe: ->(sample) { namespace_input(sample).inspect }) do |sample|
      input = namespace_input(sample)
      pr, kind, sha = input.values_at("PR_NUMBER", "NAMESPACE_KIND", "SOURCE_SHA")
      digest = Digest::SHA256.hexdigest("pr:#{pr}:kind:#{kind}:sha:#{sha}")[0, 16]
      namespace = derive.call(input)
      assert_equal "pr#{pr}-#{kind}-#{digest}", namespace
      assert_equal namespace, derive.call(input)
      assert_operator namespace.length, :<=, 63
      assert_match(/\A[a-z0-9-]+\z/, namespace)
    end
    first = namespace_input(original)
    refute_equal derive.call(first), derive.call(first.merge("SOURCE_SHA" => "b" * 40))
  end

  def namespace_input(sample)
    letters = "abcdefghijklmnopqrstuvwxyz"
    alphabet = "#{letters}0123456789-"
    kind = letters[sample[2] % letters.length] + sample[3, sample[1] % 15].map { |i| alphabet[i % alphabet.length] }.join
    {"PR_NUMBER" => (1 + sample[0] % 9_999_999_999).to_s, "NAMESPACE_KIND" => kind,
     "SOURCE_SHA" => sample[17, 40].map { |i| "0123456789abcdef"[i % 16] }.join}
  end

  def test_forge_pins_every_image_role_to_an_explicit_tag
    step = forge_step("Pin all Forge image roles to explicit tags")
    pr_tag = "pr-7_#{"c" * 40}"
    assert_equal(
      ["IMAGE_TAG=#{pr_tag}", "UPGRADE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_NAME=forge"],
      run_step_env(step, "USE_BASELINE_IMAGE" => "false", "BASELINE_IMAGE_TAG" => "", "PR_IMAGE_TAG" => pr_tag),
    )
    assert_equal(
      ["IMAGE_TAG=#{"d" * 40}", "UPGRADE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_TAG=#{pr_tag}", "FORGE_IMAGE_NAME=forge"],
      run_step_env(step, "USE_BASELINE_IMAGE" => "true", "BASELINE_IMAGE_TAG" => "d" * 40, "PR_IMAGE_TAG" => pr_tag),
    )
  end

  # Every tag template carries the PR-scoped prefix, and one pushed tag carries the full SHA.
  def test_published_tags_are_pr_scoped_and_include_the_full_sha
    bake = File.read(File.join(ROOT, "docker", "builder", "docker-bake-rust-all.hcl"))
    body = bake[/^function "generate_tags" \{\n(.*?)^\}/m, 1]
    templates = body.lines.reject { |line| line.strip.start_with?("//") }.join.scan(/"([^"]*\$\{target\}:[^"]*)"/).flatten
    refute_empty templates
    templates.each { |template| assert_includes template, "/${target}:${IMAGE_TAG_PREFIX}", template }
    assert_includes templates, "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}"
  end
end
