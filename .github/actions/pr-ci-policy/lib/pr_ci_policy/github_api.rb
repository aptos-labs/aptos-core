# frozen_string_literal: true

require "json"
require "net/http"
require "uri"

module PrCiPolicy
  class GitHubApi
    DEFAULT_RESPONSE_LIMIT = 2 * 1024 * 1024

    def initialize(base_url:, token:)
      raise PolicyError, "GitHub token is missing" unless token.is_a?(String) && !token.empty?
      @base = URI(base_url)
      raise PolicyError, "GitHub API URL must use HTTPS" unless @base.is_a?(URI::HTTPS) && @base.host
      @token = token
    rescue URI::InvalidURIError => e
      raise PolicyError, "GitHub API URL is invalid: #{e.message}"
    end

    def get_json(path, not_found: false, max_bytes: DEFAULT_RESPONSE_LIMIT)
      raise PolicyError, "GitHub API path is invalid" unless path.is_a?(String) && path.start_with?("/")
      uri = @base.dup
      uri.path = [@base.path.sub(%r{/\z}, ""), path.split("?", 2).first].join
      uri.query = path.split("?", 2)[1]
      request = Net::HTTP::Get.new(uri)
      request["Accept"] = "application/vnd.github+json"
      request["Authorization"] = "Bearer #{@token}"
      request["X-GitHub-Api-Version"] = "2022-11-28"

      body = +""
      response = nil
      Net::HTTP.start(uri.host, uri.port, use_ssl: true, open_timeout: 10, read_timeout: 30) do |http|
        http.request(request) do |result|
          response = result
          result.read_body do |chunk|
            body << chunk
            raise PolicyError, "GitHub API response exceeds size limit" if body.bytesize > max_bytes
          end
        end
      end
      return nil if response.code == "404" && not_found
      raise PolicyError, "GitHub API request failed with HTTP #{response.code}" unless response.code == "200"
      JSON.parse(body, max_nesting: 40)
    rescue JSON::ParserError => e
      raise PolicyError, "GitHub API returned malformed JSON: #{e.message}"
    rescue IOError, SystemCallError, Timeout::Error, SocketError => e
      raise PolicyError, "GitHub API request failed: #{e.class}: #{e.message}"
    end
  end
end
