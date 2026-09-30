# frozen_string_literal: true

require "minitest/autorun"
require "open3"
require "tmpdir"
require_relative "../workflow_test_helper"

class SharedPrivilegedControlTests < Minitest::Test
  include WorkflowTestHelper

  CHECKOUT_PIN = "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"
  GCP_AUTH_PIN = "google-github-actions/auth@c200f3691d83b41bf9bbd8638997a462592937ed"
  DOCKER_LOGIN_PIN = "docker/login-action@c94ce9fb468520275223c153574b00df6fe4bcc9"
  BUILDX_PIN = "docker/setup-buildx-action@8d2750c68a42422c14e847fe6c8ac0403b4cbd6f"
  EXACT_SOURCE_ACTION = "./trusted-base/.github/actions/checkout-exact-pr-source"
  GCP_REGISTRY_ACTION = "./trusted-base/.github/actions/gcp-registry-auth"
  SETUP_ACTION = "./trusted-base/.github/actions/privileged-pr-setup"
  REGISTRIES = ["us-docker.pkg.dev", "us-west1-docker.pkg.dev"].freeze
  # file => [job, first step that runs PR code]
  PRIVILEGED_WORKFLOWS = {
    "workflow-run-docker-rust-publish-pr.yaml" => ["publish-images", "Rebuild and publish immutable PR images"],
    "workflow-run-pr-e2e-tests.yaml" => ["e2e-tests", "Rebuild and publish exact-SHA test images"],
    "workflow-run-forge-pr.yaml" => ["forge", "Rebuild and publish exact-SHA Forge images"],
  }.freeze

  def test_exact_source_action_fixes_checkout_controls
    action = load_action("checkout-exact-pr-source")
    assert_required_source_inputs(action)
    action_steps = action.dig("runs", "steps")
    checkouts = action_steps.select { |step| step["uses"].to_s.start_with?("actions/checkout@") }
    assert_equal 1, checkouts.length
    checkout = checkouts.first
    assert_equal CHECKOUT_PIN, checkout.fetch("uses")
    assert_equal(
      {
        "repository" => "${{ inputs.source_repository }}",
        "ref" => "${{ inputs.source_sha }}",
        "path" => "pr-source",
        "fetch-depth" => 1,
        "persist-credentials" => false,
      },
      checkout.fetch("with"),
    )
    index = action_steps.index(checkout)
    assert_match(/verify-source\.sh" validate/, action_steps.fetch(index - 1).fetch("run"))
    assert_equal "pr-source", action_steps.fetch(index + 1).fetch("working-directory")
    assert_match(/verify-source\.sh" verify/, action_steps.fetch(index + 1).fetch("run"))
  end

  def test_exact_source_verifier_rejects_bad_shas_and_pins_branch_to_head
    verifier = File.join(ROOT, ".github", "actions", "checkout-exact-pr-source", "verify-source.sh")
    Dir.mktmpdir("exact-source") do |repo|
      git!(repo, "init", "--quiet")
      git!(repo, "config", "user.email", "ci@example.invalid")
      git!(repo, "config", "user.name", "CI")
      File.write(File.join(repo, "tracked.txt"), "trusted fixture\n")
      git!(repo, "add", "tracked.txt")
      git!(repo, "commit", "--quiet", "-m", "fixture")
      sha = git!(repo, "rev-parse", "HEAD").strip
      run = ->(*args) { Open3.capture3({"GITHUB_RUN_ID" => "24680"}, "bash", verifier, *args, chdir: repo) }

      stdout, stderr, status = run.call("verify", sha)
      assert status.success?, "#{stdout}\n#{stderr}"
      assert_equal sha, git!(repo, "rev-parse", "HEAD").strip
      assert_equal "ci-pr-24680", git!(repo, "branch", "--show-current").strip

      ["abc123", "A" * 40, "g" * 40].each do |bad_sha|
        _stdout, bad_stderr, bad_status = run.call("validate", bad_sha)
        refute bad_status.success?, bad_sha
        assert_includes bad_stderr, "full lowercase commit SHA", bad_sha
      end

      _stdout, mismatch_stderr, mismatch_status = run.call("verify", sha.start_with?("0") ? "1" * 40 : "0" * 40)
      refute mismatch_status.success?
      assert_includes mismatch_stderr, "does not match"
    end
  end

  def test_gcp_registry_action_has_fixed_auth_and_registry_boundary
    action = load_action("gcp-registry-auth")
    assert_equal %w[access_token_lifetime create_credentials_file service_account workload_identity_provider],
                 action.fetch("inputs").keys.sort
    assert_equal "false", action.fetch("inputs").fetch("create_credentials_file").fetch("default")
    action_steps = action.dig("runs", "steps")
    auth = action_steps.find { |step| step["id"] == "auth" }
    assert_equal GCP_AUTH_PIN, auth.fetch("uses")
    assert_equal "access_token", auth.fetch("with").fetch("token_format")
    assert_registry_logins(action_steps, "auth")
  end

  def test_privileged_setup_runs_every_pre_execution_control_in_order
    action = load_action("privileged-pr-setup")
    assert_required_source_inputs(action)
    assert_equal "composite", action.dig("runs", "using")
    action_steps = action.dig("runs", "steps")
    assert_equal 5, action_steps.length
    prerequisites, source, buildx, auth, tags = action_steps
    %w[GCP_WORKLOAD_IDENTITY_PROVIDER GCP_SERVICE_ACCOUNT_EMAIL GCP_DOCKER_ARTIFACT_REPO].each do |name|
      assert_includes prerequisites.fetch("run"), "test -n \"$#{name}\"", name
    end
    assert_equal EXACT_SOURCE_ACTION, source.fetch("uses")
    assert_equal({"source_repository" => "${{ inputs.source_repository }}", "source_sha" => "${{ inputs.source_sha }}"},
                 source.fetch("with"))
    assert_equal BUILDX_PIN, buildx.fetch("uses")
    assert_equal false, buildx.dig("with", "keep-state")
    assert_equal GCP_REGISTRY_ACTION, auth.fetch("uses")
    assert_equal(
      {
        "create_credentials_file" => false,
        "access_token_lifetime" => 5400,
        "workload_identity_provider" => "${{ inputs.workload_identity_provider }}",
        "service_account" => "${{ inputs.service_account }}",
      },
      auth.fetch("with"),
    )
    assert_equal "tags", tags.fetch("id")
    action_steps.filter_map { |step| step["run"] }.each { |script| refute_includes script, "${{" }
    assert_includes policy_manifest.fetch("protected_runtime_prefixes"), ".github/actions/privileged-pr-setup/"
  end

  # docker-bake-rust-all.sh runs from the PR checkout. The trusted tag must equal the
  # tag that the script publishes, so run both with stub git and docker and compare.
  def test_privileged_setup_tag_matches_the_published_tag
    sha = "0123456789abcdef0123456789abcdef01234567"
    [["release", ""], ["performance", ""], ["release", "failpoints"], ["ci", "a,b"]].each do |profile, features|
      Dir.mktmpdir("image-tag") do |dir|
        env = {"PR_NUMBER" => "42", "SOURCE_SHA" => sha, "PROFILE" => profile, "FEATURES" => features}
        outputs, github_env = run_tag_step(dir, env)
        assert_equal ["PR_IMAGE_TAG=#{outputs.fetch("image_tag")}"], github_env

        bin = File.join(dir, "bin")
        Dir.mkdir(bin)
        File.write(File.join(bin, "git"), "#!/bin/sh\n[ \"$1\" = rev-parse ] && echo #{sha}\nexit 0\n")
        File.write(File.join(bin, "docker"),
                   "#!/bin/sh\n[ \"$1 $2\" = 'buildx bake' ] && printf '%s' \"$IMAGE_TAG_PREFIX\" > \"$PREFIX_FILE\"\nexit 0\n")
        File.chmod(0o755, File.join(bin, "git"), File.join(bin, "docker"))
        bake_env = {"PATH" => "#{bin}:#{ENV.fetch("PATH")}", "CI" => "true", "PROFILE" => profile,
                    "FEATURES" => features, "CUSTOM_IMAGE_TAG_PREFIX" => outputs.fetch("custom_image_tag_prefix"),
                    "TARGET_CACHE_ID" => "pr-42-#{sha}", "PREFIX_FILE" => File.join(dir, "prefix")}
        bake = File.join(ROOT, "docker", "builder", "docker-bake-rust-all.sh")
        _stdout, stderr, status = Open3.capture3(bake_env, "bash", bake, "forge-images", chdir: dir)
        assert status.success?, stderr
        assert_equal File.read(File.join(dir, "prefix")) + sha, outputs.fetch("image_tag"), "#{profile}:#{features}"
      end
    end
  end

  def test_privileged_setup_rejects_values_that_could_inject_environment_lines
    [{"PR_NUMBER" => "1\nX=1"}, {"PROFILE" => "release\nX=1"}, {"FEATURES" => "failpoints\nX=1"}].each do |override|
      Dir.mktmpdir("image-tag") do |dir|
        env = {"PR_NUMBER" => "42", "SOURCE_SHA" => "a" * 40, "PROFILE" => "release", "FEATURES" => ""}.merge(override)
        assert_nil run_tag_step(dir, env, expect_success: false), override.keys.first
      end
    end
  end

  def test_privileged_setup_accepts_every_manifest_feature_set
    docker_manifest.fetch("variants").each do |variant|
      Dir.mktmpdir("image-tag") do |dir|
        env = {"PR_NUMBER" => "42", "SOURCE_SHA" => "a" * 40, "PROFILE" => variant.fetch("profile"),
               "FEATURES" => variant.fetch("features")}
        run_tag_step(dir, env)
      end
    end
  end

  def test_callers_load_base_owned_actions_before_pr_execution
    local_workflow = load_workflow("workflow-run-docker-rust-build-pr.yaml")
    assert_equal true, trigger(local_workflow).fetch("workflow_call").fetch("inputs").fetch("BASE_SHA").fetch("required")
    local_job = jobs(local_workflow).fetch("build-local-images")
    assert_equal({"contents" => "read"}, local_job.fetch("permissions"))
    refute local_job.key?("environment")
    trusted_index = assert_trusted_checkout(local_job)
    source = exact_source_step(local_job)
    assert_equal EXACT_SOURCE_ACTION, source.fetch("uses")
    assert_equal({"source_repository" => "${{ inputs.SOURCE_REPOSITORY }}", "source_sha" => "${{ inputs.SOURCE_SHA }}"},
                 source.fetch("with"))
    assert_operator trusted_index, :<, steps(local_job).index(source)

    PRIVILEGED_WORKFLOWS.each do |file, (job_name, first_source_step)|
      job = jobs(load_workflow(file)).fetch(job_name)
      job_steps = steps(job)
      trusted_index = assert_trusted_checkout(job)
      setups = job_steps.select { |step| step["uses"] == SETUP_ACTION }
      assert_equal 1, setups.length, file
      setup = setups.first
      assert_equal "setup", setup.fetch("id"), file
      %w[SOURCE_REPOSITORY SOURCE_SHA PR_NUMBER].each do |input|
        assert_equal "${{ inputs.#{input} }}", setup.dig("with", input.downcase), "#{file}: #{input}"
      end
      assert_nil exact_source_step(job), file
      refute job_steps.any? { |step| step["uses"] == GCP_REGISTRY_ACTION }, file

      execution = job_steps.find { |step| step["name"] == first_source_step }
      assert_operator trusted_index, :<, job_steps.index(setup), file
      assert_operator job_steps.index(setup), :<, job_steps.index(execution), file
      assert_equal "pr-source", execution.fetch("working-directory"), file
      assert_equal "${{ steps.setup.outputs.custom_image_tag_prefix }}", execution.dig("env", "CUSTOM_IMAGE_TAG_PREFIX"), file
      # No step may load PR-owned action code: the job mints registry credentials
      # before PR code runs, so a "./pr-source/..." step would hand it those credentials.
      refute job_steps.any? { |step| step["uses"].to_s.start_with?("./pr-source/") }, file
      # Before PR code runs, every local action must come from the trusted checkout.
      before_execution = job_steps.take(job_steps.index(execution))
      before_execution.select { |step| step["uses"].to_s.start_with?("./") }.each do |step|
        assert step.fetch("uses").start_with?("./trusted-base/"), "#{file}: #{step["name"]}"
      end
      # PR code can rewrite trusted-base/ once it runs, so no local action may load afterwards.
      later = job_steps.drop(job_steps.index(execution) + 1)
      assert_empty later.select { |step| step["uses"].to_s.start_with?("./") }, file
    end
  end

  # Forge runs only the images that this job rebuilt from the exact SHA. It must not
  # discover recent images, wait for images from other runs, or post PR comments.
  def test_forge_uses_only_its_own_protected_images
    source = load_workflow("workflow-run-forge-pr.yaml").to_s
    %w[find_recent_images wait-images-ci sticky-pull-request-comment].each { |text| refute_includes source, text }
  end

  def test_callers_bind_the_event_base_sha
    callers = jobs(load_workflow("docker-build-test.yaml")).select do |_name, job|
      (PRIVILEGED_WORKFLOWS.keys + ["workflow-run-docker-rust-build-pr.yaml"]).include?(File.basename(job["uses"].to_s))
    end
    refute_empty callers
    callers.each do |name, job|
      assert_equal "${{ github.event.pull_request.base.sha }}", job.fetch("with").fetch("BASE_SHA"), name
    end
  end

  # These refreshes run after PR code, which could have rewritten trusted-base/,
  # so they use pinned public actions directly instead of gcp-registry-auth.
  def test_post_execution_credential_refreshes_use_direct_pinned_actions
    refreshes = {
      "workflow-run-pr-e2e-tests.yaml" => ["gcp-test-auth", false, 5400, "Generate YAML API specification"],
      "workflow-run-forge-pr.yaml" => [
        "gcp-forge-auth", true, "${{ steps.auth-duration.outputs.value }}", "Run Forge with protected exact-SHA images",
      ],
    }
    refreshes.each do |file, (auth_id, credentials_file, lifetime, consumer_name)|
      job_name, first_source_step = PRIVILEGED_WORKFLOWS.fetch(file)
      job_steps = steps(jobs(load_workflow(file)).fetch(job_name))
      auth = job_steps.find { |step| step["id"] == auth_id }
      assert_equal GCP_AUTH_PIN, auth.fetch("uses"), file
      assert_equal(
        {
          "create_credentials_file" => credentials_file,
          "token_format" => "access_token",
          "access_token_lifetime" => lifetime,
          "workload_identity_provider" => "${{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}",
          "service_account" => "${{ secrets.GCP_SERVICE_ACCOUNT_EMAIL }}",
        },
        auth.fetch("with"),
        file,
      )
      execution_index = job_steps.index { |step| step["name"] == first_source_step }
      assert_operator execution_index, :<, job_steps.index(auth), file
      assert_registry_logins(job_steps, auth_id)
      # The refreshed token and registry logins must exist before their first consumer.
      consumer_index = job_steps.index { |step| step["name"] == consumer_name }
      refute_nil consumer_index, file
      assert_operator job_steps.index(auth), :<, consumer_index, file
      job_steps.each_with_index.select { |step, _| step["uses"].to_s.start_with?("docker/login-action@") }.each do |_, index|
        assert_operator index, :<, consumer_index, file
      end
    end
    publish = jobs(load_workflow("workflow-run-docker-rust-publish-pr.yaml")).fetch("publish-images")
    assert_empty steps(publish).select { |step| step["uses"].to_s.start_with?("google-github-actions/auth@") }
  end

  private

  # A10 classifies a local checkout-shaped step as PR-controlled by these exact input
  # names, so both actions that accept an exact PR SHA must keep them required.
  def assert_required_source_inputs(action)
    %w[source_repository source_sha].each do |name|
      assert_equal true, action.fetch("inputs").fetch(name).fetch("required"), name
    end
  end

  def git!(repo, *args)
    stdout, stderr, status = Open3.capture3("git", *args, chdir: repo)
    assert status.success?, stderr
    stdout
  end

  # Runs the tag step of privileged-pr-setup. Returns [outputs, GITHUB_ENV lines], or nil on failure.
  def run_tag_step(dir, env, expect_success: true)
    tags = load_action("privileged-pr-setup").dig("runs", "steps").find { |step| step["id"] == "tags" }
    output = File.join(dir, "output")
    github_env = File.join(dir, "env")
    full_env = env.merge("PATH" => ENV.fetch("PATH"), "GITHUB_OUTPUT" => output, "GITHUB_ENV" => github_env)
    _stdout, stderr, status = Open3.capture3(full_env, "bash", "-c", tags.fetch("run"))
    unless expect_success
      refute status.success?
      refute File.exist?(github_env)
      return nil
    end
    assert status.success?, stderr
    [File.read(output).lines(chomp: true).to_h { |line| line.split("=", 2) }, File.read(github_env).lines(chomp: true)]
  end

  def assert_registry_logins(action_steps, auth_id)
    logins = action_steps.select { |step| step["uses"].to_s.start_with?("docker/login-action@") }
    assert_equal REGISTRIES, logins.map { |step| step.dig("with", "registry") }
    auth_index = action_steps.index { |step| step["id"] == auth_id }
    logins.each do |login|
      assert_equal DOCKER_LOGIN_PIN, login.fetch("uses")
      assert_equal "oauth2accesstoken", login.dig("with", "username")
      assert_equal "${{ steps.#{auth_id}.outputs.access_token }}", login.dig("with", "password")
      assert_operator auth_index, :<, action_steps.index(login)
    end
  end

  # Asserts one checkout of the event base into trusted-base/ and returns its index.
  def assert_trusted_checkout(job)
    checkouts = checkout_steps(job)
    assert_equal 1, checkouts.length
    trusted = checkouts.first
    assert_equal CHECKOUT_PIN, trusted.fetch("uses")
    assert_equal "trusted-base", trusted.dig("with", "path")
    assert_equal "${{ github.repository }}", trusted.dig("with", "repository")
    assert_equal "${{ inputs.BASE_SHA }}", trusted.dig("with", "ref")
    assert_equal false, trusted.dig("with", "persist-credentials")
    steps(job).index(trusted)
  end
end
