# frozen_string_literal: true

require "base64"
require "uri"

module PrCiPolicy
  class Runner
    MAX_CHANGED_FILES = 1_000
    MAX_WORKFLOW_FILES = 50
    MAX_FILE_BYTES = 512 * 1024
    MAX_TOTAL_BYTES = 4 * 1024 * 1024
    PAGE_SIZE = 100
    REPOSITORY = %r{\A[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\z}
    SHA = /\A[0-9a-f]{40}\z/

    def initialize(api, manifest: Manifest.read, checker: PolicyChecker.new(manifest: manifest))
      @api = api
      @manifest = manifest
      @checker = checker
    end

    def check(event)
      context = validate_event(event)
      files = fetch_changed_files(context)
      protected_runtime_files = files.select do |file|
        @manifest.protected_runtime?(file["filename"]) || @manifest.protected_runtime?(file["previous_filename"])
      end
      unless protected_runtime_files.empty?
        return protected_runtime_files.map do |file|
          paths = [file["previous_filename"], file.fetch("filename")].compact.uniq.join(" -> ")
          Violation.new(path: paths, category: :protected_runtime)
        end
      end

      workflow_files = files.select do |file|
        workflow_path?(file["filename"]) || workflow_path?(file["previous_filename"])
      end
      raise PolicyError, "pull request changes too many workflow files" if workflow_files.length > MAX_WORKFLOW_FILES

      total_bytes = 0
      pairs = workflow_files.map do |file|
        path = file.fetch("filename")
        status = file.fetch("status")
        raise PolicyError, "workflow renames are ambiguous and are not allowed" if status == "renamed"

        base_text, head_text = fetch_pair(context, path, status)
        total_bytes += [base_text, head_text].compact.sum(&:bytesize)
        raise PolicyError, "combined workflow data exceeds size limit" if total_bytes > MAX_TOTAL_BYTES
        [path, base_text, head_text]
      end

      head_contents = pairs.to_h { |path, _base, head| [path, head] }
      callee_cache = {}
      resolver = lambda do |target|
        path = target.delete_prefix("./")
        callee_cache[path] ||= begin
          text = head_contents[path]
          unless text
            text = fetch_content(context[:head_repo], path, context[:head_sha], not_found: false)
            total_bytes += text.bytesize
            raise PolicyError, "combined workflow data exceeds size limit" if total_bytes > MAX_TOTAL_BYTES
          end
          WorkflowAnalysis.new("#{path}@head", text)
        end
      end

      pairs.flat_map do |path, base_text, head_text|
        @checker.check_pair(path, base_text, head_text, callee_resolver: resolver)
      end
    end

    private

    def validate_event(event)
      raise PolicyError, "event payload must be an object" unless event.is_a?(Hash)
      pr = event["pull_request"]
      repo = event.dig("repository", "full_name")
      base_repo = pr&.dig("base", "repo", "full_name")
      head_repo = pr&.dig("head", "repo", "full_name")
      number = pr&.dig("number")
      changed_files = pr&.dig("changed_files")
      base_sha = pr&.dig("base", "sha")
      head_sha = pr&.dig("head", "sha")

      raise PolicyError, "invalid base repository" unless repo.is_a?(String) && repo.match?(REPOSITORY) && base_repo == repo
      raise PolicyError, "invalid head repository" unless head_repo.is_a?(String) && head_repo.match?(REPOSITORY)
      raise PolicyError, "invalid pull request number" unless number.is_a?(Integer) && number.positive?
      raise PolicyError, "invalid changed_files count" unless changed_files.is_a?(Integer) && changed_files.between?(0, MAX_CHANGED_FILES)
      raise PolicyError, "invalid base SHA" unless base_sha.is_a?(String) && base_sha.match?(SHA)
      raise PolicyError, "invalid head SHA" unless head_sha.is_a?(String) && head_sha.match?(SHA)

      { base_repo: repo, head_repo: head_repo, number: number, changed_files: changed_files, base_sha: base_sha, head_sha: head_sha }
    end

    def fetch_changed_files(context)
      files = []
      page = 1
      loop do
        path = "/repos/#{context[:base_repo]}/pulls/#{context[:number]}/files?per_page=#{PAGE_SIZE}&page=#{page}"
        result = @api.get_json(path, max_bytes: 2 * 1024 * 1024)
        raise PolicyError, "pull request files response must be a list" unless result.is_a?(Array)
        raise PolicyError, "pull request file page exceeds page size" if result.length > PAGE_SIZE
        result.each { |file| validate_changed_file(file) }
        files.concat(result)
        raise PolicyError, "pull request file listing exceeds count limit" if files.length > MAX_CHANGED_FILES
        break if result.length < PAGE_SIZE
        page += 1
      end
      unless files.length == context[:changed_files]
        raise PolicyError, "pull request file listing is truncated or changed during evaluation"
      end
      filenames = files.map { |file| file.fetch("filename") }
      raise PolicyError, "pull request file listing contains duplicate paths" unless filenames.uniq.length == filenames.length
      files
    end

    def validate_changed_file(file)
      raise PolicyError, "changed file entry must be an object" unless file.is_a?(Hash)
      filename = file["filename"]
      status = file["status"]
      raise PolicyError, "changed file path is invalid" unless valid_path?(filename)
      raise PolicyError, "unknown changed file status" unless %w[added modified removed renamed].include?(status)
      if status == "renamed"
        previous = file["previous_filename"]
        raise PolicyError, "renamed file is missing previous_filename" unless valid_path?(previous)
      end
    end

    def valid_path?(path)
      path.is_a?(String) && !path.empty? && !path.include?("\0") && !path.start_with?("/") && !path.split("/").include?("..")
    end

    def workflow_path?(path)
      path.is_a?(String) && path.match?(Manifest::WORKFLOW)
    end

    def fetch_pair(context, path, status)
      base = fetch_content(context[:base_repo], path, context[:base_sha], not_found: status == "added")
      head = fetch_content(context[:head_repo], path, context[:head_sha], not_found: status == "removed")
      check_presence(status, base, head)
      [base, head]
    end

    def check_presence(status, base, head)
      case status
      when "added"
        raise PolicyError, "added workflow already exists at base SHA" unless base.nil?
        raise PolicyError, "added workflow is absent at head SHA" if head.nil?
      when "modified"
        raise PolicyError, "modified workflow is absent at base or head SHA" if base.nil? || head.nil?
      when "removed"
        raise PolicyError, "removed workflow is absent at base SHA" if base.nil?
        raise PolicyError, "removed workflow still exists at head SHA" unless head.nil?
      end
    end

    def fetch_content(repository, path, sha, not_found:)
      encoded_path = path.split("/").map { |part| URI.encode_www_form_component(part) }.join("/")
      endpoint = "/repos/#{repository}/contents/#{encoded_path}?ref=#{sha}"
      payload = @api.get_json(endpoint, not_found: not_found, max_bytes: MAX_FILE_BYTES * 2)
      return nil if payload.nil?
      raise PolicyError, "workflow content response must be an object" unless payload.is_a?(Hash)
      raise PolicyError, "workflow content is not a regular file" unless payload["type"] == "file"
      raise PolicyError, "workflow content encoding must be base64" unless payload["encoding"] == "base64"
      raise PolicyError, "workflow blob SHA is invalid" unless payload["sha"].is_a?(String) && payload["sha"].match?(SHA)
      size = payload["size"]
      raise PolicyError, "workflow size is invalid" unless size.is_a?(Integer) && size.between?(0, MAX_FILE_BYTES)
      encoded = payload["content"]
      raise PolicyError, "workflow content is missing" unless encoded.is_a?(String)
      decoded = Base64.strict_decode64(encoded.gsub(/[\r\n]/, ""))
      raise PolicyError, "workflow content size mismatch" unless decoded.bytesize == size
      decoded.force_encoding(Encoding::UTF_8)
    rescue ArgumentError => e
      raise PolicyError, "workflow content is malformed base64: #{e.message}"
    end
  end
end
