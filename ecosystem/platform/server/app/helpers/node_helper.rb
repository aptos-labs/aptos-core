# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'resolv'
require 'uri'
require 'maxmind/geoip2'
require 'httparty'

# @param [String] hostname
def normalize_hostname!(hostname)
  hostname.strip!
  hostname.downcase!
  hostname.delete_prefix! 'http://'
  hostname.delete_prefix! 'https://'
  hostname.delete_suffix! '/'
end

# @param [String] metrics
# @return MetricsResult
def extract_metrics(metrics)
  return MetricsResult.new(false, nil, 'Metrics result is empty') unless metrics.present?

  metrics.split("\n").each_entry do |metric|
    next if metric.start_with? '#'

    name, value = metric.split
    # aptos_consensus_last_committed_version 8299
    return MetricsResult.new(true, value.to_i, nil) if name == 'aptos_consensus_last_committed_version'
  end

  MetricsResult.new(false, nil, 'could not find `aptos_consensus_last_committed_version` metric')
end

VerifyResult = Struct.new(:valid, :message)
MetricsResult = Struct.new(:ok, :version, :message)
LocationResult = Struct.new(:ok, :message, :record)
IPResult = Struct.new(:ok, :ip, :message)

module NodeHelper
  class NodeVerifier
    # @param [String] hostname
    # @param [Integer] metrics_port
    def initialize(hostname, metrics_port, http_api_port)
      normalize_hostname!(hostname)

      @hostname = hostname
      @metrics_port = metrics_port
      @http_api_port = http_api_port
      @ip = resolve_ip
    end

    # @return [IPResult] ip
    attr_reader :ip

    # @return IPResult
    def resolve_ip
      return IPResult.new(true, @hostname, nil) if @hostname =~ Resolv::IPv4::Regex

      resolved_ip = Resolv::DNS.open do |dns|
        dns.timeouts = 0.5
        dns.getaddress @hostname
      end
      IPResult.new(true, resolved_ip, nil)
    rescue StandardError => e
      IPResult.new(false, nil, "DNS error: #{e}")
    end

    # @return [LocationResult]
    def location
      return LocationResult(false, "Can not fetch location with no IP: #{@ip.message}", nil) unless @ip.ok

      client = MaxMind::GeoIP2::Client.new(
        account_id: ENV.fetch('MAXMIND_ACCOUNT_ID'),
        license_key: ENV.fetch('MAXMIND_LICENSE_KEY')
      )
      LocationResult.new(true, nil, client.insights(@ip.ip))
    rescue StandardError => e
      Sentry.capture_exception(e)
      LocationResult.new(false, "Error: #{e}", nil)
    end

    # @return MetricsResult
    def fetch_metrics
      res = HTTParty.get("http://#{@hostname}:#{@metrics_port}/metrics", open_timeout: 1, read_timeout: 2,
                                                                         max_retries: 0)
      extract_metrics(res.body)
    rescue Net::ReadTimeout => e
      MetricsResult.new(false, nil, "Read timeout: #{e}")
    rescue Net::OpenTimeout => e
      MetricsResult.new(false, nil, "Open timeout: #{e}")
    rescue StandardError => e
      Sentry.capture_exception(e)
      MetricsResult.new(false, nil, "Error: #{e}")
    end

    # @return VerifyResult
    def verify_metrics
      res1 = fetch_metrics
      return VerifyResult.new(false, "Could not verify metrics; #{res1.message}") unless res1.ok

      # Sleep to allow their node to produce more versions
      sleep 1

      res2 = fetch_metrics
      return VerifyResult.new(false, "Could not verify metrics; #{res2.message}") unless res2.ok

      unless res2.version > res1.version
        return VerifyResult.new(false,
                                'Metrics last synced version did not increase. Ensure your node is running, and retry.')
      end

      VerifyResult.new(true, 'Metrics verified successfully!')
    end

    # @return [Array<VerifyResult>]
    def verify
      validations = []
      validations << verify_metrics
      validations
    end
  end
end
