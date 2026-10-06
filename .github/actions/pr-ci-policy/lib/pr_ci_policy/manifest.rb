# frozen_string_literal: true

require "json"
require "set"

module PrCiPolicy
  Manifest = Data.define(:hardened_workflows, :approved_protected_reusables, :protected_runtime_prefixes)

  # Policy path lists from .github/ci/pr-ci-policy.json. The checker reads the
  # copy at the trusted base SHA; the file is itself under a protected prefix.
  class Manifest
    PATH = File.expand_path("../../../../ci/pr-ci-policy.json", __dir__)
    KEYS = %w[approved_protected_reusables hardened_workflows protected_runtime_prefixes].freeze
    WORKFLOW = %r{\A\.github/workflows/[^/]+\.ya?ml\z}
    # One path segment: not empty, not `.` or `..`.
    SEGMENT = %r{(?!\.\.?(?:/|\z))[A-Za-z0-9_.-]+}
    # A repo-relative directory prefix (ends in `/`) or exact file.
    PREFIX = %r{\A(?:#{SEGMENT}/)*#{SEGMENT}/?\z}

    def self.read(path = PATH)
      data = JSON.parse(File.read(path), max_nesting: 3)
      unless data.is_a?(Hash) && data.keys.sort == KEYS
        raise PolicyError, "policy manifest must contain exactly #{KEYS.join(", ")}"
      end

      new(
        hardened_workflows: entries(data, "hardened_workflows", WORKFLOW),
        approved_protected_reusables: entries(data, "approved_protected_reusables", WORKFLOW),
        protected_runtime_prefixes: entries(data, "protected_runtime_prefixes", PREFIX),
      )
    rescue JSON::ParserError, SystemCallError => e
      raise PolicyError, "policy manifest is unreadable: #{e.message}"
    end

    def self.entries(data, key, pattern)
      values = data.fetch(key)
      valid = values.is_a?(Array) && !values.empty? && values.uniq.length == values.length &&
              values.all? { |value| value.is_a?(String) && value.match?(pattern) }
      raise PolicyError, "policy manifest #{key} must be a non-empty list of unique paths matching #{pattern.source}" unless valid

      values.to_set.freeze
    end
    private_class_method :entries

    def hardened?(path)
      hardened_workflows.include?(path)
    end

    # `uses:` must name the local file exactly, as "./.github/workflows/<file>".
    def approved_reusable?(target)
      target.is_a?(String) && target.start_with?("./") && approved_protected_reusables.include?(target.delete_prefix("./"))
    end

    def protected_runtime?(path)
      path.is_a?(String) && protected_runtime_prefixes.any? do |prefix|
        prefix.end_with?("/") ? path.start_with?(prefix) : path == prefix
      end
    end
  end
end
