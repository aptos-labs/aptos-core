# frozen_string_literal: true

require "set"

module PrCiPolicy
  # Compares one workflow file at base and head. Three tiers apply, strictest
  # first: the policy workflow itself, the manifest's hardened workflows, and a
  # ratchet for every other workflow.
  class PolicyChecker
    POLICY_WORKFLOW_PATH = ".github/workflows/pr-ci-policy.yaml"

    def initialize(manifest: Manifest.read)
      @manifest = manifest
    end

    def check_pair(path, base_text, head_text, callee_resolver: nil)
      if head_text.nil?
        protected_path = path == POLICY_WORKFLOW_PATH || @manifest.hardened?(path)
        return protected_path ? [Violation.new(path: path, category: :removed)] : []
      end
      if path == POLICY_WORKFLOW_PATH && SafeYaml.load(base_text, "#{path}@base") != SafeYaml.load(head_text, "#{path}@head")
        return [Violation.new(path: path, category: :policy_workflow_changed)]
      end

      head = WorkflowAnalysis.new("#{path}@head", head_text)
      base = base_text && WorkflowAnalysis.new("#{path}@base", base_text)
      if @manifest.hardened?(path)
        hardened_violations(path, base, head)
      else
        ratchet_violations(path, base, head, callee_resolver)
      end
    end

    private

    # A hardened workflow may not change its execution state, any risky
    # PR-controlled job (an anchor), or any job that an anchor depends on
    # through `needs`. An upstream job can steer an anchor through its outputs
    # and its result, so a change to it is reported once for each privilege
    # of each dependent anchor. A job that is itself an anchor is reported
    # only as an anchor.
    def hardened_violations(path, base, head)
      state_changed = !base.nil? && base.workflow_execution_state != head.workflow_execution_state
      anchors = head.jobs.select { |_name, job| job.pr_controlled? && !job.privileges.empty? }
      dependents = Hash.new { |hash, key| hash[key] = [] }
      anchors.each_key { |anchor| head.upstream_of(anchor).each { |upstream| dependents[upstream] << anchor } }

      head.jobs.flat_map do |name, job|
        next [] unless anchors.key?(name) || dependents.key?(name)
        next [] unless state_changed || base&.jobs&.[](name)&.raw != job.raw

        if anchors.key?(name)
          category = state_changed ? :hardened_state_changed : :hardened_job_changed
          next job_violations(path, name, job, job.privileges, category)
        end

        category = state_changed ? :hardened_state_changed : :hardened_upstream_changed
        dependents.fetch(name).sort.flat_map do |anchor|
          job_violations(path, name, anchors.fetch(anchor), anchors.fetch(anchor).privileges, category, dependent: anchor)
        end
      end
    end

    # Other workflows may keep existing debt, but a PR-controlled job may not
    # appear, become PR-controlled, gain privilege, or gain a PR source. The
    # approved environment does not exempt a job here: approval does not make
    # PR code safe to run with credentials.
    def ratchet_violations(path, base, head, callee_resolver)
      head.jobs.flat_map do |name, job|
        next [] unless job.pr_controlled?

        privileges = unprotected_privileges(job, callee_resolver)
        next [] if privileges.empty?

        base_job = base&.jobs&.[](name)
        next job_violations(path, name, job, privileges, :new_job) if base_job.nil?
        next job_violations(path, name, job, privileges, :new_execution) unless base_job.pr_controlled?

        introduced = privileges - unprotected_privileges(base_job, callee_resolver)
        next job_violations(path, name, job, introduced, :privilege_increase) unless introduced.empty?
        next job_violations(path, name, job, privileges, :new_source) unless (job.sources - base_job.sources).empty?

        []
      end
    end

    # The one privilege function for base and head jobs: the job's privileges,
    # or none when it only delegates OIDC to a protected callee.
    def unprotected_privileges(job, callee_resolver)
      privileges = job.privileges
      delegated_privilege_is_protected?(job, privileges, callee_resolver) ? Set.new : privileges
    end

    def delegated_privilege_is_protected?(job, privileges, callee_resolver)
      target = job.reusable_target
      return false unless @manifest.approved_reusable?(target)
      return false unless privileges.map(&:kind) == [:id_token_write]
      return false unless callee_resolver

      callee = callee_resolver.call(target)
      return false unless callee.is_a?(WorkflowAnalysis) && callee.workflow_call?
      return false if callee.jobs.empty?

      callee.jobs.values.all?(&:fixed_environment)
    end

    # `job` holds the sources and privileges. For an upstream finding it is
    # the dependent anchor, and `name` is the changed upstream job.
    def job_violations(path, name, job, privileges, category, dependent: nil)
      privileges.sort_by(&:to_s).map do |privilege|
        Violation.new(path: path, category: category, job: name, sources: job.sources, privilege: privilege, dependent: dependent)
      end
    end
  end
end
