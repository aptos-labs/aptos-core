# frozen_string_literal: true

require "pathname"
require "set"

module PrCiPolicy
  # `sources` and `privileges` split the job's risks by dimension. `needs`
  # holds the names of the jobs this job directly depends on.
  JobAnalysis = Data.define(:raw, :sources, :privileges, :fixed_environment, :reusable_target, :needs)

  class JobAnalysis
    def pr_controlled?
      !sources.empty?
    end
  end

  class WorkflowAnalysis
    APPROVED_ENVIRONMENT = "privileged-pr-ci"
    WORKFLOW_EXECUTION_KEYS = %w[on permissions env defaults cache-mode].freeze
    PULL_REQUEST_CONTEXT_EVENTS = %w[pull_request pull_request_target workflow_call].freeze
    EXECUTION_FIELDS = %w[
      concurrency container continue-on-error defaults env environment if
      outputs permissions runs-on secrets services strategy steps timeout-minutes
      uses with
    ].freeze
    WRITE_LEVEL = /\Awrite(?:-all)?\z/i
    # Trusted local wrappers that run actions/checkout on `source_repository`
    # at `source_sha`, for example ./trusted-base/.github/actions/checkout-exact-pr-source.
    EXACT_SOURCE_ACTION = %r{\A\./(?:[^/]+/)*\.github/actions/(?:checkout-exact-pr-source|privileged-pr-setup)/?\z}i
    SOURCE_MEMBER = "(?:git_)?(?:sha|ref|repository|repo|head_sha|head_ref)"

    # `root.a.b` or `root['a']["b"]`, any spacing, case-insensitive. Members are
    # regex fragments. A dot-form member must end at an identifier boundary; a
    # bracket-form member ends at its `]`. The boundary is `(?!\w)`, not
    # `(?![\w-])`: this builder also feeds the actions/github-script rules,
    # where `-` is a subtraction operator, not part of an identifier, so
    # `context.issue.number-1` must still match `context.issue.number`.
    def self.member_access(root, *members)
      chain = members.map { |member| "\\s*(?:\\.\\s*#{member}(?!\\w)|\\[\\s*['\"]#{member}['\"]\\s*\\])" }.join
      Regexp.new("\\b#{root}#{chain}", Regexp::IGNORECASE)
    end

    PR_VALUE = Regexp.union(
      member_access("github", "event", "pull_request", "head", "(?:sha|ref|repo)"),
      member_access("github", "head_ref"),
    )
    UNTRUSTED_INPUT = member_access("inputs", SOURCE_MEMBER)
    DYNAMIC_SOURCE_OUTPUT = member_access("needs", "[A-Za-z0-9_-]+", "outputs", SOURCE_MEMBER)
    TRUSTED_PR_EXECUTION_VALUE = Regexp.union(
      member_access("github", "event", "pull_request", "base", "sha"),
      member_access("github", "event", "pull_request", "base", "repo", "full_name"),
    )

    # `input` names the strings a rule reads: step `uses`, step `run`, every
    # execution value, or execution values with trusted base references removed.
    Rule = Data.define(:input, :pattern, :risk, :pull_request_context_only)

    def self.rule(input, pattern, risk, pull_request_context_only: false)
      Rule.new(input: input, pattern: pattern, risk: risk, pull_request_context_only: pull_request_context_only)
    end

    EXECUTION_VALUE = Risk.new(kind: :execution_value)
    SHELL_SOURCE = Risk.new(kind: :shell_source)
    SCRIPT_API_SOURCE = Risk.new(kind: :script_api_source)
    RULES = [
      rule(:uses, %r{\A(?:aws-actions/configure-aws-credentials|google-github-actions/auth|azure/login|docker/login-action|hashicorp/vault-action)@}i,
           Risk.new(kind: :cloud_auth)),
      rule(:uses, %r{\A(?:google-github-actions/get-secretmanager-secrets|hashicorp/vault-action|aws-actions/aws-secretsmanager-get-secrets)@}i,
           Risk.new(kind: :secret_manager)),
      rule(:uses, %r{\A(?:peter-evans/repository-dispatch|marocchino/sticky-pull-request-comment|actions/github-script)@}i,
           Risk.new(kind: :write_authority)),
      rule(:run, /\b(?:gcloud\s+auth|aws\s+(?:configure|sts\s+assume-role)|az\s+login|docker\s+login)\b/i,
           Risk.new(kind: :cloud_auth)),
      rule(:run, /\b(?:gcloud\s+secrets|aws\s+secretsmanager|az\s+keyvault|vault\s+(?:read|kv))\b/i,
           Risk.new(kind: :secret_manager)),
      rule(:run, /\bgh\s+(?:pr\s+comment|api\b[^\n]*(?:dispatches|comments))\b/i,
           Risk.new(kind: :write_authority)),

      rule(:untrusted_execution, member_access("github", "event", "pull_request"), EXECUTION_VALUE),
      rule(:untrusted_execution, PR_VALUE, EXECUTION_VALUE),
      rule(:untrusted_execution, UNTRUSTED_INPUT, EXECUTION_VALUE),
      rule(:untrusted_execution, member_access("github", "event", "number"), EXECUTION_VALUE, pull_request_context_only: true),
      rule(:untrusted_execution, member_access("github", "event_path"), EXECUTION_VALUE, pull_request_context_only: true),

      rule(:execution, /(?:\$\{\s*(?:env:)?\s*github_(?:event_path|head_ref)\b[^}]*\}|\$(?:env:)?github_(?:event_path|head_ref)\b|%github_(?:event_path|head_ref)%)/i,
           SHELL_SOURCE, pull_request_context_only: true),
      rule(:execution, %r{\brefs/pull/|\bgit\b[^;&|]*?\bfetch\b[^;&|]*?\bpull/}i, SHELL_SOURCE),
      rule(:execution, /\bgh\b[^;&|]*?\bpr\s+checkout\b/i, SHELL_SOURCE),
      rule(:execution, %r{\bgh\b[^;&|]*?\bapi\b[^;&|]*?(?:/pulls?\b|\bgraphql\b[^;&|]*?\bpullrequest\s*\()}i, SHELL_SOURCE),
      rule(:execution, %r{\b(?:curl|wget|invoke-restmethod|invoke-webrequest)\b[^;&|]*?(?:github_api_url|github\.api_url|api\.github\.com)[^;&|]*?/pulls?\b}i,
           SHELL_SOURCE),

      rule(:execution, member_access("context", "issue", "number"), SCRIPT_API_SOURCE, pull_request_context_only: true),
      rule(:execution, /(?:\bprocess\s*\.\s*env\s*\.\s*github_(?:event_path|head_ref)\b|\bos\s*\.\s*environ\s*\[\s*['"]github_(?:event_path|head_ref)['"]\s*\]|\benv\s*\[\s*['"]github_(?:event_path|head_ref)['"]\s*\])/i,
           SCRIPT_API_SOURCE, pull_request_context_only: true),
      rule(:execution, member_access("context", "payload", "pull_request"), SCRIPT_API_SOURCE),
      rule(:execution, member_access("(?:github|octokit)", "rest", "pulls"), SCRIPT_API_SOURCE),
      rule(:execution, %r{\b(?:github|octokit)\s*\.\s*rest\s*\.\s*git\s*\.\s*getref\b[^;&|]*?\bref\s*:\s*['"](?:refs/)?pull/}i, SCRIPT_API_SOURCE),
      rule(:execution, %r{\b(?:github|octokit)\s*\.\s*request\b[^;&|]*?/pulls?\b}i, SCRIPT_API_SOURCE),
      rule(:execution, /\b(?:github|octokit)\s*\.\s*graphql\b[^;&|]*?\bpullrequest\s*\(/i, SCRIPT_API_SOURCE),
    ].freeze

    attr_reader :events, :jobs, :workflow_execution_state

    def initialize(path, text)
      @path = path
      @document = SafeYaml.load(text, path)
      @events = event_names(@document["on"])
      @workflow_permissions = @document["permissions"]
      @workflow_env = @document["env"]
      @workflow_defaults = @document["defaults"]
      @workflow_cache_mode = @document["cache-mode"]
      @workflow_cache_mode_configured = @document.key?("cache-mode")
      @workflow_execution_state = @document.select do |key, _value|
        WORKFLOW_EXECUTION_KEYS.include?(key)
      end
      raw_jobs = @document["jobs"]
      raise PolicyError, "#{path}: jobs must be a mapping" unless raw_jobs.is_a?(Hash)

      @jobs = raw_jobs.to_h do |name, raw|
        raise PolicyError, "#{path}: job #{name.inspect} must be a mapping" unless name.is_a?(String) && raw.is_a?(Hash)
        [name, analyze_job(name, raw)]
      end
      validate_needs_graph
    end

    def workflow_call?
      @events.include?("workflow_call")
    end

    # Every job that `name` depends on through `needs`, directly or
    # transitively. The result does not contain `name`.
    def upstream_of(name)
      upstream = Set.new
      pending = @jobs.fetch(name).needs.to_a
      until pending.empty?
        current = pending.pop
        pending.concat(@jobs.fetch(current).needs.to_a) if upstream.add?(current)
      end
      upstream
    end

    private

    def event_names(value)
      case value
      when String
        Set[value]
      when Array
        raise PolicyError, "#{@path}: workflow events must be strings" unless value.all? { |entry| entry.is_a?(String) }
        Set.new(value)
      when Hash
        raise PolicyError, "#{@path}: workflow event names must be strings" unless value.keys.all? { |entry| entry.is_a?(String) }
        Set.new(value.keys)
      else
        raise PolicyError, "#{@path}: on must be a string, list, or mapping"
      end
    end

    def analyze_job(name, raw)
      needs = parse_needs(name, raw["needs"])
      risks = Set.new
      inherited_scope = effective_inherited_scope(raw)

      if @events.include?("pull_request_target") && !job_excludes_pull_request_target?(raw["if"])
        risks << Risk.new(kind: :pull_request_target_workflow)
      end
      risks << Risk.new(kind: :pull_request_workflow) if @events.include?("pull_request")
      # A workflow_run job runs with default-branch privileges after a run that
      # a fork PR can trigger. Its payload and artifacts are PR-controlled.
      risks << Risk.new(kind: :workflow_run_workflow) if @events.include?("workflow_run")
      uses, runs = analyze_steps(raw, risks)
      analyze_reusable_call(raw, risks)
      execution = EXECUTION_FIELDS.flat_map { |key| strings(raw[key]) } + strings(inherited_scope)
      inputs = {
        uses: uses,
        run: runs,
        execution: execution,
        untrusted_execution: execution.map { |value| value.gsub(TRUSTED_PR_EXECUTION_VALUE, "") },
      }
      apply_rules(inputs, risks)
      analyze_permissions(raw.key?("permissions") ? raw["permissions"] : @workflow_permissions, risks)
      cache_mode = raw.key?("cache-mode") ? raw["cache-mode"] : @workflow_cache_mode
      cache_mode_configured = raw.key?("cache-mode") || @workflow_cache_mode_configured
      analyze_cache_mode(cache_mode, cache_mode_configured, risks)
      fixed_environment = raw.key?("environment") && analyze_environment(raw["environment"], risks)
      analyze_secrets(raw, inherited_scope, risks)

      sources, privileges = risks.partition(&:source?).map(&:to_set)
      JobAnalysis.new(
        raw: raw,
        sources: sources,
        privileges: privileges,
        fixed_environment: fixed_environment,
        reusable_target: raw["uses"],
        needs: needs,
      )
    end

    def parse_needs(name, value)
      names = case value
              when nil then []
              when String then [value]
              when Array then value
              else raise PolicyError, "#{@path}: job #{name.inspect} needs must be a string or list of strings"
              end
      raise PolicyError, "#{@path}: job #{name.inspect} needs must be a string or list of strings" unless names.all?(String)

      names.to_set.freeze
    end

    # GitHub rejects a workflow whose `needs` names an unknown job or forms a
    # cycle. The checker rejects it too, so `upstream_of` always terminates.
    # Kahn's algorithm keeps the check iterative for any job count.
    def validate_needs_graph
      dependents = Hash.new { |hash, key| hash[key] = [] }
      @jobs.each do |name, job|
        job.needs.each do |dependency|
          raise PolicyError, "#{@path}: job #{name.inspect} needs unknown job #{dependency.inspect}" unless @jobs.key?(dependency)

          dependents[dependency] << name
        end
      end
      unresolved = @jobs.transform_values { |job| job.needs.size }
      ready = unresolved.select { |_name, count| count.zero? }.keys
      until ready.empty?
        dependents[ready.pop].each do |dependent|
          unresolved[dependent] -= 1
          ready << dependent if unresolved[dependent].zero?
        end
      end
      cyclic = unresolved.select { |_name, count| count.positive? }.keys.sort
      raise PolicyError, "#{@path}: needs graph contains a cycle (unresolved jobs: #{cyclic.join(", ")})" unless cyclic.empty?
    end

    def apply_rules(inputs, risks)
      RULES.each do |rule|
        next if rule.pull_request_context_only && !pull_request_event_context?
        next unless inputs.fetch(rule.input).any? { |text| text.match?(rule.pattern) }

        risks << rule.risk
      end
    end

    def job_excludes_pull_request_target?(condition)
      return false unless condition.is_a?(String)

      tokens = condition_tokens(condition)
      return false if tokens.nil? || double_negation?(tokens)

      conjuncts = Expression.conjuncts(Expression.strip_parentheses(tokens))
      return false unless conjuncts

      conjuncts.any? { |conjunct| event_exclusion?(Expression.strip_parentheses(conjunct)) }
    end

    # A job `if:` is one whole ${{ }} template or a bare expression. GitHub
    # renders any other mix of text and ${{ }} as a non-empty string, which is
    # always true, so such a condition excludes nothing.
    def condition_tokens(condition)
      return Expression.whole_template(condition) if condition.start_with?("${{")

      Expression.tokenize(condition) unless condition.include?("${{")
    rescue Expression::Unparseable
      nil
    end

    def double_negation?(tokens)
      tokens.each_cons(2).any? { |first, second| Expression.symbol?(first, "!") && second.type == :symbol && second.value.start_with?("!") }
    end

    # `github.event_name != 'pull_request_target'`, or `github.event_name ==`
    # any other non-empty event name.
    def event_exclusion?(tokens)
      length = Expression.member_chain_length(tokens, %w[github event_name])
      return false unless length && tokens.length == length + 2

      operator = tokens[length]
      literal = tokens[length + 1]
      return false unless literal.type == :string && literal.value.ascii_only?

      target = literal.value.casecmp?("pull_request_target")
      (Expression.symbol?(operator, "!=") && target) ||
        (Expression.symbol?(operator, "==") && !target && !literal.value.empty?)
    end

    def effective_inherited_scope(job)
      {
        "defaults" => merge_inherited_mapping(@workflow_defaults, job["defaults"]),
        "env" => merge_inherited_mapping(@workflow_env, job["env"]),
      }
    end

    def merge_inherited_mapping(workflow_value, job_value)
      return workflow_value unless job_value.is_a?(Hash)
      return job_value unless workflow_value.is_a?(Hash)

      workflow_value.merge(job_value) do |_key, inherited, override|
        if inherited.is_a?(Hash) && override.is_a?(Hash)
          merge_inherited_mapping(inherited, override)
        else
          override
        end
      end
    end

    # Validates the steps, records checkout source risks, and returns the step
    # `uses` and `run` strings for the rule table.
    def analyze_steps(job, risks)
      steps = job["steps"]
      return [[], []] if steps.nil?
      raise PolicyError, "#{@path}: steps must be a list" unless steps.is_a?(Array)

      uses = []
      runs = []
      steps.each_with_index do |step, index|
        raise PolicyError, "#{@path}: step #{index + 1} must be a mapping" unless step.is_a?(Hash)
        if step.key?("uses")
          action = step["uses"]
          raise PolicyError, "#{@path}: step uses must be a string" unless action.is_a?(String)
          if action.downcase.start_with?("actions/checkout@")
            analyze_checkout(step, risks)
          elsif local_exact_source_action?(action)
            analyze_exact_source_checkout(step, risks)
          end
          uses << action
        end
        if step.key?("run")
          command = step["run"]
          raise PolicyError, "#{@path}: step run must be a string" unless command.is_a?(String)
          runs << command
        end
      end
      [uses, runs]
    end

    # A local `uses:` path can carry redundant segments (`//`, `/.`, `/x/..`)
    # that a runner still resolves to the same file. Only local paths
    # (`./...`) are normalized; other `uses:` values are matched as-is.
    def local_exact_source_action?(action)
      return false unless action.start_with?("./")

      normalized = "./#{Pathname.new(action).cleanpath}"
      normalized.match?(EXACT_SOURCE_ACTION)
    end

    def analyze_checkout(step, risks)
      with = step["with"] || {}
      raise PolicyError, "#{@path}: checkout with must be a mapping" unless with.is_a?(Hash)

      risks << Risk.new(kind: :pull_request_checkout_default) if @events.include?("pull_request") && with["ref"].nil?
      classify_checkout_source(with, "repository", "ref", risks)
    end

    # The wrappers pass both inputs straight to actions/checkout, so the inputs
    # are classified exactly like checkout's `repository` and `ref`.
    def analyze_exact_source_checkout(step, risks)
      with = step["with"]
      raise PolicyError, "#{@path}: exact-source checkout with must be a mapping" unless with.is_a?(Hash)
      %w[source_repository source_sha].each do |key|
        raise PolicyError, "#{@path}: exact-source checkout #{key} must be a string" unless with[key].is_a?(String)
      end

      classify_checkout_source(with, "source_repository", "source_sha", risks)
    end

    def classify_checkout_source(with, repository_key, ref_key, risks)
      repository = with[repository_key]
      ref = with[ref_key]
      if repository.is_a?(String)
        if repository.match?(PR_VALUE) || repository.match?(UNTRUSTED_INPUT) || (repository.include?("${{") && !trusted_repository_expression?(repository))
          risks << Risk.new(kind: :checkout_repository)
        end
      elsif with.key?(repository_key)
        raise PolicyError, "#{@path}: checkout #{repository_key} must be a string"
      end

      if ref.is_a?(String)
        if ref.match?(PR_VALUE) || ref.match?(UNTRUSTED_INPUT) || (ref.include?("${{") && !trusted_ref_expression?(ref))
          risks << Risk.new(kind: :checkout_ref)
        end
      elsif with.key?(ref_key)
        raise PolicyError, "#{@path}: checkout #{ref_key} must be a string"
      end
    end

    def trusted_ref_expression?(value)
      normalized = value.gsub(/\s+/, "")
      ["${{github.event.pull_request.base.sha}}", "${{github.sha}}"].include?(normalized)
    end

    def trusted_repository_expression?(value)
      normalized = value.gsub(/\s+/, "")
      ["${{github.repository}}", "${{github.event.pull_request.base.repo.full_name}}"].include?(normalized)
    end

    def analyze_reusable_call(job, risks)
      return unless job.key?("uses")
      raise PolicyError, "#{@path}: reusable workflow uses must be a string" unless job["uses"].is_a?(String)
      with = job["with"] || {}
      raise PolicyError, "#{@path}: reusable workflow with must be a mapping" unless with.is_a?(Hash)
      if strings(with).any? { |value| value.match?(PR_VALUE) || value.match?(UNTRUSTED_INPUT) || value.match?(DYNAMIC_SOURCE_OUTPUT) }
        risks << Risk.new(kind: :reusable_input)
      end
    end

    def pull_request_event_context?
      @events.any? { |event| PULL_REQUEST_CONTEXT_EVENTS.include?(event) }
    end

    def analyze_permissions(value, risks)
      if value.nil?
        risks << Risk.new(kind: :implicit_permissions)
      elsif value.is_a?(String)
        risks << Risk.new(kind: :write_permission, detail: "write-all") if value.match?(WRITE_LEVEL)
        risks << Risk.new(kind: :dynamic_permissions) if value.include?("${{")
      elsif value.is_a?(Hash)
        value.each do |scope, level|
          raise PolicyError, "#{@path}: permission scope must be a string" unless scope.is_a?(String)
          raise PolicyError, "#{@path}: permission level must be a string" unless level.is_a?(String)
          if level.include?("${{")
            risks << Risk.new(kind: :dynamic_permissions)
          elsif level.casecmp("write").zero?
            if scope.casecmp("id-token").zero?
              risks << Risk.new(kind: :id_token_write)
            else
              risks << Risk.new(kind: :write_permission, detail: "#{scope}: write")
            end
          elsif !%w[read none].include?(level.downcase)
            risks << Risk.new(kind: :unknown_permission_level, detail: "#{scope}: #{level}")
          end
        end
      else
        raise PolicyError, "#{@path}: permissions must be a string or mapping"
      end
    end

    def analyze_cache_mode(value, configured, risks)
      return unless configured
      return if value.is_a?(String) && %w[read none].include?(value)

      risks << Risk.new(kind: :cache_write)
    end

    def analyze_environment(value, risks)
      name = if value.is_a?(String)
               value
             elsif value.is_a?(Hash)
               unknown = value.keys - %w[name url]
               risks << Risk.new(kind: :unknown_environment_configuration) unless unknown.empty?
               value["name"]
             else
               raise PolicyError, "#{@path}: environment must be a string or mapping"
             end
      raise PolicyError, "#{@path}: environment name must be a string" unless name.is_a?(String)

      if name.include?("${{")
        risks << Risk.new(kind: :dynamic_environment)
        false
      elsif name != APPROVED_ENVIRONMENT
        risks << Risk.new(kind: :unapproved_environment, detail: name)
        false
      else
        true
      end
    end

    def analyze_secrets(job, inherited_scope, risks)
      strings([inherited_scope, job]).each do |value|
        secret_references(value).each do |reference|
          risks << Risk.new(kind: :secret, detail: reference)
        end
      end
      risks << Risk.new(kind: :inherited_secrets) if job["secrets"].is_a?(String) && job["secrets"].casecmp("inherit").zero?
    end

    # An expression that cannot be tokenized fails closed as a secret reference.
    def secret_references(value)
      Expression.embedded(value).flat_map { |tokens| secret_references_in(tokens, value) }
    rescue Expression::Unparseable
      ["unparseable expression"]
    end

    def secret_references_in(tokens, text)
      brackets = Expression.matching_pairs(tokens, "[", "]")
      tokens.each_with_index.filter_map do |token, index|
        next unless token.type == :identifier && token.value.casecmp?("secrets")
        next if index.positive? && Expression.symbol?(tokens[index - 1], ".")

        following = tokens[index + 1]
        if Expression.symbol?(following, ".")
          member = tokens[index + 2]
          if member&.type == :identifier
            "secrets.#{member.value.upcase}"
          elsif Expression.symbol?(member, "*")
            "secrets context"
          end
        elsif Expression.symbol?(following, "[")
          closing = brackets[index + 1]
          next unless closing

          inner = tokens[(index + 2)...closing]
          if inner.length == 1 && inner.first.type == :string
            "secrets.#{inner.first.value.upcase}"
          else
            dynamic = text.byteslice(following.finish, tokens[closing].start - following.finish).gsub(/\s+/, " ").strip.downcase
            "secrets[#{dynamic}]" unless dynamic.empty?
          end
        else
          "secrets context"
        end
      end
    end

    def strings(value)
      case value
      when String then [value]
      when Array then value.flat_map { |child| strings(child) }
      when Hash then value.flat_map { |key, child| strings(key) + strings(child) }
      else []
      end
    end
  end
end
