# frozen_string_literal: true

require "minitest/autorun"
require "fileutils"
require "open3"
require "tmpdir"
require_relative "../workflow_test_helper"
require_relative "../pr_ci_policy/property_support"

class SharedPrivilegedControlTests < Minitest::Test
  include WorkflowTestHelper
  include PolicyPropertySupport

  EXACT_SOURCE_ACTION = "./trusted-base/.github/actions/checkout-exact-pr-source"
  GCP_REGISTRY_ACTION = "./trusted-base/.github/actions/gcp-registry-auth"
  REFRESHED_GCP_REGISTRY_ACTION = "./trusted-refresh/.github/actions/gcp-registry-auth"
  RUNTIME_SETUP_ACTION = "./trusted-base/.github/actions/privileged-pr-runtime-setup"
  REGISTRIES = ["us-docker.pkg.dev", "us-west1-docker.pkg.dev"].freeze
  # file => job that checks out and runs PR code
  PR_CODE_JOBS = {
    "workflow-run-docker-rust-build-pr.yaml" => "build-local-images",
    "workflow-run-docker-rust-publish-pr.yaml" => "build-images",
    "workflow-run-pr-e2e-tests.yaml" => "e2e-tests",
  }.freeze

  def test_exact_source_action_fixes_checkout_controls
    action = load_action("checkout-exact-pr-source")
    assert_required_source_inputs(action)
    action_steps = action.dig("runs", "steps")
    checkouts = action_steps.select { |step| step["uses"].to_s.start_with?("actions/checkout@") }
    assert_equal 1, checkouts.length
    checkout = checkouts.first
    assert_equal "pr-source", action.fetch("inputs").fetch("path").fetch("default")
    assert_equal "1", action.fetch("inputs").fetch("fetch-depth").fetch("default")
    assert_equal(
      {
        "repository" => "${{ inputs.source_repository }}",
        "ref" => "${{ inputs.source_sha }}",
        "path" => "${{ inputs.path }}",
        "fetch-depth" => "${{ inputs.fetch-depth }}",
        "persist-credentials" => false,
        "allow-unsafe-pr-checkout" => true,
      },
      checkout.fetch("with"),
    )
    # The verifier is copied out of the checkout path before the PR checkout, so PR
    # files cannot replace it, and it runs after the checkout.
    validate = action_steps.index { |step| step["run"].to_s.include?('verify-source.sh" validate') }
    stage = action_steps.index { |step| step["id"] == "stage-verifier" }
    verify = action_steps.index { |step| step["run"].to_s.include?("steps.stage-verifier.outputs.path") }
    order = [validate, stage, action_steps.index(checkout), verify]
    refute_includes order, nil
    assert_equal order.sort, order
    assert_includes action_steps.fetch(stage).fetch("run"), "RUNNER_TEMP"
    assert_equal "${{ inputs.path }}", action_steps.fetch(verify).fetch("working-directory")
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

      _stdout, mismatch_stderr, mismatch_status = run.call("verify", sha.start_with?("0") ? "1" * 40 : "0" * 40)
      refute mismatch_status.success?
      assert_includes mismatch_stderr, "does not match"
    end
  end

  def test_staged_verifier_survives_replacing_a_root_checkout
    stage = load_action("checkout-exact-pr-source").dig("runs", "steps").find { |step| step["id"] == "stage-verifier" }
    Dir.mktmpdir("exact-source-stage") do |dir|
      action_dir = File.join(dir, "action")
      Dir.mkdir(action_dir)
      FileUtils.cp(File.join(ROOT, ".github/actions/checkout-exact-pr-source/verify-source.sh"), action_dir)
      output = File.join(dir, "step-output")
      stage_env = {"RUNNER_TEMP" => dir, "GITHUB_ACTION_PATH" => action_dir, "GITHUB_OUTPUT" => output}
      _stdout, stderr, status = Open3.capture3(stage_env, "bash", "-c", stage.fetch("run"))
      assert status.success?, stderr
      verifier = File.read(output).split("=", 2).last.strip
      FileUtils.rm_rf(action_dir)

      repo = File.join(dir, "replacement")
      Dir.mkdir(repo)
      git!(repo, "init", "--quiet")
      git!(repo, "config", "user.email", "ci@example.invalid")
      git!(repo, "config", "user.name", "CI")
      File.write(File.join(repo, "tracked.txt"), "replacement source\n")
      git!(repo, "add", "tracked.txt")
      git!(repo, "commit", "--quiet", "-m", "source")
      sha = git!(repo, "rev-parse", "HEAD").strip
      _stdout, verify_stderr, verify_status = Open3.capture3({"GITHUB_RUN_ID" => "24681"}, "bash", verifier, "verify", sha, chdir: repo)
      assert verify_status.success?, verify_stderr
      assert_equal sha, git!(repo, "rev-parse", "HEAD").strip
    end
  end

  def test_exact_source_validator_accepts_generated_lowercase_full_shas
    check_property("valid-source-shas", corpus: [[0] * 5, [0xaaaaaaaa] * 5, [0xffffffff] * 5],
                   generate: ->(random) { Array.new(5) { random.rand(2**32) } },
                   describe: ->(words) { source_sha(words).inspect }) do |words|
      sha = source_sha(words)
      stdout, stderr, status = validate_source_sha(sha)
      assert status.success?, "#{sha}: #{stdout}\n#{stderr}"
    end
  end

  def test_exact_source_validator_rejects_invalid_sha_categories
    # Every category stays invalid when the integer tuple shrinks.
    check_property("invalid-source-shas", corpus: (0..7).map { |category| [category, 0, 0] },
                   generate: ->(random) { [random.rand(8), random.rand(64), random.rand(40)] },
                   describe: ->(sample) { invalid_source_sha(sample).inspect }) do |sample|
      sha = invalid_source_sha(sample)
      _stdout, stderr, status = validate_source_sha(sha)
      refute status.success?, sha.inspect
      assert_includes stderr, "full lowercase commit SHA", sha.inspect
    end
  end

  def test_gcp_registry_action_has_fixed_auth_and_registry_boundary
    action = load_action("gcp-registry-auth")
    assert_equal %w[access_token_lifetime create_credentials_file service_account workload_identity_provider],
                 action.fetch("inputs").keys.sort
    assert_equal "false", action.fetch("inputs").fetch("create_credentials_file").fetch("default")
    action_steps = action.dig("runs", "steps")
    auth = action_steps.find { |step| step["id"] == "auth" }
    assert auth.fetch("uses").start_with?("google-github-actions/auth@")
    assert_equal "access_token", auth.fetch("with").fetch("token_format")
    assert_registry_logins(action_steps, "auth")
  end

  def test_privileged_setup_runs_every_pre_execution_control_in_order
    runtime = load_action("privileged-pr-runtime-setup")
    assert_required_source_inputs(runtime)
    assert_equal "composite", runtime.dig("runs", "using")
    action_steps = runtime.dig("runs", "steps")
    assert_equal 3, action_steps.length
    prerequisites, auth, tags = action_steps
    %w[GCP_WORKLOAD_IDENTITY_PROVIDER GCP_SERVICE_ACCOUNT_EMAIL GCP_DOCKER_ARTIFACT_REPO].each do |name|
      assert_includes prerequisites.fetch("run"), "test -n \"$#{name}\"", name
    end
    refute action_steps.any? { |step| step.fetch("uses", "") == EXACT_SOURCE_ACTION }
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
  end

  # The trusted setup computes the tag. The PR checkout's bake script must use
  # that exact prefix for every configured build variant.
  def test_privileged_setup_tag_matches_the_published_tag
    sha = "0123456789abcdef0123456789abcdef01234567"
    tag_variants.each do |profile, features|
      assert_published_tag(tag_env.merge("SOURCE_SHA" => sha, "PROFILE" => profile, "FEATURES" => features), bake: true)
    end
    corpus = tag_variants.each_index.map { |index| [index, 41, 0, 0, 0, 0, *[0xaaaaaaaa] * 5, 24679, 1] }
    check_property("published-image-tags", corpus: corpus, generate: lambda { |random|
      [tag_variants.length, random.rand(9_999_999_999), random.rand(65), random.rand(2**384),
       random.rand(65), random.rand(2**384), *Array.new(5) { random.rand(2**32) },
       random.rand(10**18), random.rand(10**6)]
    }, describe: ->(sample) { generated_tag_env(sample).inspect }) do |sample|
      assert_published_tag(generated_tag_env(sample))
    end
  end

  def test_bake_script_preserves_an_explicit_empty_prefix
    Dir.mktmpdir("image-tag") do |dir|
      assert_equal "", run_stubbed_bake(dir, "a" * 40, "performance", "failpoints", "")
    end
  end

  def test_bake_script_uses_local_helper_when_prefix_is_unset
    Dir.mktmpdir("image-tag") do |dir|
      bake = File.join(dir, "docker-bake-rust-all.sh")
      File.write(bake, File.read(File.join(ROOT, "docker", "builder", "docker-bake-rust-all.sh")))
      File.write(File.join(dir, "image-tag-prefix.sh"), "image_tag_prefix() { printf 'from-local-helper_'; }\n")
      assert_equal "from-local-helper_", run_stubbed_bake(dir, "a" * 40, "performance", "consensus-only-perf-test", nil, bake)
    end
  end

  def test_privileged_setup_rejects_values_that_could_inject_environment_lines
    corpus = (0..4).to_a.product((0..2).to_a, [0])
    check_property("environment-line-injection", corpus: corpus,
                   generate: ->(random) { [random.rand(5), random.rand(3), random.rand(64)] },
                   describe: ->(sample) { tag_env.merge(injected_tag_input(sample)).inspect }) do |sample|
      assert_tag_rejected(injected_tag_input(sample))
    end
  end

  def test_privileged_setup_rejects_invalid_tag_field_categories
    corpus = tag_input_fields.each_index.flat_map do |field|
      invalid_tag_values(tag_input_fields.fetch(field), 0).each_index.map { |category| [field, category, 0] }
    end
    check_property("invalid-tag-fields", corpus: corpus, generate: lambda { |random|
      field = random.rand(tag_input_fields.length)
      [field, random.rand(invalid_tag_values(tag_input_fields.fetch(field), 0).length), random.rand(64)]
    }, describe: ->(sample) { tag_env.merge(invalid_tag_input(sample)).inspect }) do |sample|
      assert_tag_rejected(invalid_tag_input(sample))
    end
  end

  def test_pr_code_jobs_hold_no_authority_and_load_trusted_base_first
    PR_CODE_JOBS.each do |file, job_name|
      job = jobs(load_workflow(file)).fetch(job_name)
      assert_equal({"contents" => "read"}, job.fetch("permissions"), file)
      refute job.key?("environment"), file
      %w[secrets. id-token gcp-registry-auth docker/login-action privileged-pr-runtime-setup GIT_CREDENTIALS
         cache-from cache-to --push].each { |token| refute_includes job.to_s, token, "#{file}: #{token}" }
      source = exact_source_step(job)
      assert_equal EXACT_SOURCE_ACTION, source.fetch("uses"), file
      assert_equal({"source_repository" => "${{ inputs.SOURCE_REPOSITORY }}", "source_sha" => "${{ inputs.SOURCE_SHA }}"},
                   source.fetch("with").slice("source_repository", "source_sha"), file)
      assert_operator assert_trusted_checkout(job), :<, steps(job).index(source), file
    end
  end

  # The local build has no path to publish or share what it builds.
  def test_local_build_has_no_publish_path
    workflow = load_workflow("workflow-run-docker-rust-build-pr.yaml")
    assert_equal({"contents" => "read"}, workflow.fetch("permissions"))
    assert_equal true, trigger(workflow).dig("workflow_call", "inputs", "BASE_SHA", "required")
    source = workflow.to_s
    %w[environment artifact].each { |text| refute_includes source, text }
    %w[CI=false TARGET_REGISTRY=local].each { |text| assert_includes source, text }
  end

  # The credentialed Forge job runs only trusted-base code and uses PR identity as data.
  def test_forge_runs_trusted_setup_before_trusted_execution
    job = jobs(load_workflow("workflow-run-forge-pr.yaml")).fetch("forge")
    job_steps = steps(job)
    trusted_index = assert_trusted_checkout(job)
    setups = job_steps.select { |step| step["uses"] == RUNTIME_SETUP_ACTION }
    assert_equal 1, setups.length
    setup = setups.first
    assert_equal "setup", setup.fetch("id")
    %w[SOURCE_REPOSITORY SOURCE_SHA PR_NUMBER].each do |input|
      assert_equal "${{ inputs.#{input} }}", setup.dig("with", input.downcase), input
    end
    registry_refreshes = job_steps.select { |step| step["uses"] == REFRESHED_GCP_REGISTRY_ACTION }
    assert_equal 1, registry_refreshes.length

    execution = job_steps.find { |step| step["name"] == "Run pre-Forge checks with explicit image tags" }
    assert_operator trusted_index, :<, job_steps.index(setup)
    assert_operator job_steps.index(setup), :<, job_steps.index(execution)
    assert_equal "trusted-base", execution.fetch("working-directory")
    refute job_steps.any? { |step| step["run"].to_s.include?("docker-bake-rust-all.sh") }
    registry_refreshes.each { |step| assert_operator job_steps.index(step), :<, job_steps.index(execution) }
    refute execution.fetch("env", {}).key?("CUSTOM_IMAGE_TAG_PREFIX")
    # Local actions must come from an exact trusted checkout.
    job_steps.take(job_steps.index(execution)).select { |step| step["uses"].to_s.start_with?("./") }.each do |step|
      assert_match %r{\A\./trusted-(?:base|refresh)/}, step.fetch("uses"), step["name"]
    end
  end

  # Forge consumes the protected producer manifest. It must not discover recent
  # images, wait for images from other runs, or post PR comments.
  def test_forge_uses_only_its_own_protected_images
    source = load_workflow("workflow-run-forge-pr.yaml").to_s
    %w[find_recent_images wait-images-ci sticky-pull-request-comment].each { |text| refute_includes source, text }
  end

  def test_forge_refresh_reuses_the_registry_action_before_trusted_scripts
    job_steps = steps(jobs(load_workflow("workflow-run-forge-pr.yaml")).fetch("forge"))
    auth = job_steps.find { |step| step["id"] == "gcp-forge-auth" }
    assert_equal REFRESHED_GCP_REGISTRY_ACTION, auth.fetch("uses")
    refreshed = job_steps.find { |step| step["name"] == "Refresh exact trusted base actions" }
    assert refreshed.fetch("uses").start_with?("actions/checkout@")
    assert_equal "${{ inputs.BASE_SHA }}", refreshed.dig("with", "ref")
    assert_equal "trusted-refresh", refreshed.dig("with", "path")
    assert_equal true, auth.dig("with", "create_credentials_file")
    assert_equal "${{ steps.auth-duration.outputs.value }}", auth.dig("with", "access_token_lifetime")
    assert_equal "${{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}", auth.dig("with", "workload_identity_provider")
    assert_equal "${{ secrets.GCP_SERVICE_ACCOUNT_EMAIL }}", auth.dig("with", "service_account")
    setup_index = job_steps.index { |step| step["uses"] == RUNTIME_SETUP_ACTION }
    auth_index = job_steps.index(auth)
    forge_index = job_steps.index { |step| step["name"] == "Run pre-Forge checks with explicit image tags" }
    assert_operator setup_index, :<, auth_index
    assert_operator setup_index, :<, job_steps.index(refreshed)
    assert_operator job_steps.index(refreshed), :<, auth_index
    assert_operator auth_index, :<, forge_index
    assert_empty job_steps.select { |step| step["uses"].to_s.start_with?("docker/login-action@") }
    token_export = job_steps.find { |step| step.dig("env", "ACCESS_TOKEN") == "${{ steps.gcp-forge-auth.outputs.access_token }}" }
    refute_nil token_export
    assert_includes token_export.fetch("run"), "CLOUDSDK_AUTH_ACCESS_TOKEN=$ACCESS_TOKEN"
    assert_operator auth_index, :<, job_steps.index(token_export)
    assert_operator job_steps.index(token_export), :<, forge_index
  end

  private

  def source_sha(words)
    words.map { |word| word.to_s(16).rjust(8, "0") }.join
  end

  def validate_source_sha(sha)
    Open3.capture3("bash", File.join(ROOT, ".github/actions/checkout-exact-pr-source/verify-source.sh"), "validate", sha)
  end

  def invalid_source_sha(sample)
    category, length, position = sample
    sha = "a" * 40
    case category
    when 0 then "a" * (length % 40)
    when 1 then "a" * (41 + length % 24)
    when 2 then sha.dup.tap { |value| value[position] = "A" }
    when 3 then sha.dup.tap { |value| value[position] = "g" }
    when 4 then sha + "\r"
    when 5 then sha + "\n"
    when 6 then sha + "\r\nX=1"
    when 7 then sha.dup.tap { |value| value[position] = "é" }
    end
  end

  def tag_env
    {"PR_NUMBER" => "42", "SOURCE_SHA" => "a" * 40, "PROFILE" => "release", "FEATURES" => "",
     "GITHUB_RUN_ID" => "24680", "GITHUB_RUN_ATTEMPT" => "2"}
  end

  def tag_variants
    @tag_variants ||= docker_manifest.fetch("variants").map { |variant| [variant.fetch("profile"), variant.fetch("features")] } + [["ci", "a,b"]]
  end

  def generated_tag_env(sample)
    variant, pr, profile_length, profile_code, features_length, features_code, *rest = sample
    profile, features = tag_variants.fetch(variant) do
      [profile_length.zero? ? "release" : choice_string(profile_code, profile_length, "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-"),
       choice_string(features_code, features_length, "abcdefghijklmnopqrstuvwxyz0123456789,-")]
    end
    tag_env.merge("PR_NUMBER" => (pr + 1).to_s, "PROFILE" => profile, "FEATURES" => features,
                  "SOURCE_SHA" => source_sha(rest.take(5)), "GITHUB_RUN_ID" => (rest.fetch(5) + 1).to_s,
                  "GITHUB_RUN_ATTEMPT" => (rest.fetch(6) + 1).to_s)
  end

  def choice_string(code, length, alphabet)
    Array.new(length) do
      code, character = code.divmod(alphabet.length)
      alphabet[character]
    end.join
  end

  def assert_published_tag(env, bake: false)
    # Test-owned rules: omit release, normalize each feature separator, retain full SHA.
    prefix = "pr-#{env.fetch('PR_NUMBER')}_"
    prefix += "#{env.fetch('PROFILE')}_" unless env.fetch("PROFILE") == "release"
    prefix += "#{env.fetch('FEATURES').tr(',-', '__')}_" unless env.fetch("FEATURES").empty?
    prefix += "r#{env.fetch('GITHUB_RUN_ID')}-a#{env.fetch('GITHUB_RUN_ATTEMPT')}_"
    Dir.mktmpdir("image-tag") do |dir|
      outputs, github_env = run_tag_step(dir, env)
      assert_equal prefix, outputs.fetch("image_tag_prefix"), env.inspect
      assert_equal "#{prefix}#{env.fetch('SOURCE_SHA')}", outputs.fetch("image_tag"), env.inspect
      assert_equal ["PR_IMAGE_TAG=#{outputs.fetch('image_tag')}"], github_env
      if bake
        assert_equal prefix, run_stubbed_bake(dir, env.fetch("SOURCE_SHA"), env.fetch("PROFILE"), env.fetch("FEATURES"), prefix)
      end
    end
  end

  def tag_input_fields
    %w[PR_NUMBER PROFILE FEATURES GITHUB_RUN_ID GITHUB_RUN_ATTEMPT]
  end

  def invalid_tag_values(name, length)
    numeric = ["", "0", "01", "-1", "+1", "a" * (length + 1), "1_2", " 1", "1 ", "１"]
    case name
    when "PR_NUMBER" then numeric + ["1" * 11]
    when "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT" then numeric
    when "PROFILE" then ["", "a.b", "a/b", "a,b", "a b", "a=b", "é" * (length + 1)]
    when "FEATURES" then ["A" * (length + 1), "a_b", "a.b", "a/b", "a b", "a=b", "é" * (length + 1)]
    end
  end

  def assert_tag_rejected(override)
    Dir.mktmpdir("image-tag") do |dir|
      assert_nil run_tag_step(dir, tag_env.merge(override), expect_success: false), override.inspect
    end
  end

  def injected_tag_input(sample)
    field, separator, length = sample
    name = tag_input_fields.fetch(field)
    newline = ["\r", "\n", "\r\n"].fetch(separator)
    {name => "#{tag_env.fetch(name)}#{newline}X=#{'1' * (length + 1)}"}
  end

  def invalid_tag_input(sample)
    field, category, length = sample
    name = tag_input_fields.fetch(field)
    values = invalid_tag_values(name, length)
    {name => values.fetch(category % values.length)}
  end

  def run_stubbed_bake(dir, sha, profile, features, prefix, bake = File.join(ROOT, "docker", "builder", "docker-bake-rust-all.sh"))
    bin = File.join(dir, "bin")
    Dir.mkdir(bin)
    File.write(File.join(bin, "git"), "#!/bin/sh\n[ \"$1\" = rev-parse ] && echo #{sha}\nexit 0\n")
    File.write(File.join(bin, "docker"),
               "#!/bin/sh\n[ \"$1 $2\" = 'buildx bake' ] && printf '%s' \"$IMAGE_TAG_PREFIX\" > \"$PREFIX_FILE\"\nexit 0\n")
    File.chmod(0o755, File.join(bin, "git"), File.join(bin, "docker"))
    env = {"PATH" => "#{bin}:#{ENV.fetch("PATH")}", "CI" => "true", "PROFILE" => profile,
           "FEATURES" => features, "IMAGE_TAG_PREFIX" => prefix, "TARGET_CACHE_ID" => "pr-42-#{sha}",
           "PREFIX_FILE" => File.join(dir, "prefix")}
    _stdout, stderr, status = Open3.capture3(env, "bash", bake, "forge-images", chdir: dir)
    assert status.success?, stderr
    File.read(File.join(dir, "prefix"))
  end

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

  # Runs the shared runtime tag step. Returns [outputs, GITHUB_ENV lines], or nil on failure.
  def run_tag_step(dir, env, expect_success: true)
    tags = load_action("privileged-pr-runtime-setup").dig("runs", "steps").find { |step| step["id"] == "tags" }
    output = File.join(dir, "output")
    github_env = File.join(dir, "env")
    full_env = env.merge("PATH" => ENV.fetch("PATH"), "GITHUB_OUTPUT" => output, "GITHUB_ENV" => github_env,
                         "GITHUB_ACTION_PATH" => File.join(ROOT, ".github", "actions", "privileged-pr-runtime-setup"))
    _stdout, stderr, status = Open3.capture3(full_env, "bash", "-c", tags.fetch("run"))
    unless expect_success
      refute status.success?
      refute File.exist?(github_env)
      refute File.exist?(output)
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
      assert_equal "oauth2accesstoken", login.dig("with", "username")
      assert_equal "${{ steps.#{auth_id}.outputs.access_token }}", login.dig("with", "password")
      assert_operator auth_index, :<, action_steps.index(login)
    end
  end

  # Asserts the initial checkout of the event base into trusted-base/.
  def assert_trusted_checkout(job)
    checkouts = checkout_steps(job)
    assert_operator checkouts.length, :>=, 1
    trusted = checkouts.first
    assert_equal "trusted-base", trusted.dig("with", "path")
    assert_equal "${{ github.repository }}", trusted.dig("with", "repository")
    assert_equal "${{ inputs.BASE_SHA }}", trusted.dig("with", "ref")
    assert_equal false, trusted.dig("with", "persist-credentials")
    steps(job).index(trusted)
  end
end
