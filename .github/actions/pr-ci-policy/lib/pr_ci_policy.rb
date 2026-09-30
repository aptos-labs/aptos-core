# frozen_string_literal: true

require "json"
require_relative "pr_ci_policy/safe_yaml"
require_relative "pr_ci_policy/manifest"
require_relative "pr_ci_policy/findings"
require_relative "pr_ci_policy/expression"
require_relative "pr_ci_policy/workflow_analysis"
require_relative "pr_ci_policy/policy_checker"
require_relative "pr_ci_policy/runner"
require_relative "pr_ci_policy/github_api"

module PrCiPolicy
  class PolicyError < StandardError; end

  def self.run_from_environment
    raise PolicyError, "policy must run only for pull_request_target" unless ENV["GITHUB_EVENT_NAME"] == "pull_request_target"
    event_path = ENV.fetch("GITHUB_EVENT_PATH")
    raise PolicyError, "event payload is too large" if File.size(event_path) > 2 * 1024 * 1024
    event = JSON.parse(File.read(event_path), max_nesting: 50)
    api = GitHubApi.new(base_url: ENV.fetch("GITHUB_API_URL", "https://api.github.com"), token: ENV.fetch("GITHUB_TOKEN"))
    violations = Runner.new(api).check(event)
    if violations.empty?
      puts "PR CI policy check passed."
      return 0
    end
    warn "PR CI policy check rejected the proposed workflow changes:"
    violations.each { |violation| warn "- #{violation}" }
    1
  rescue KeyError, JSON::ParserError, Errno::ENOENT, PolicyError => e
    warn "PR CI policy check failed closed: #{e.message}"
    1
  end
end
