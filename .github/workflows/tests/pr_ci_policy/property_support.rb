# frozen_string_literal: true

require "zlib"
ENV["RANTLY_VERBOSE"] ||= "0"
require "rantly/property"
require "rantly/shrinks"

# Integer choice tuples keep generation and shrinking inside each test's grammar.
# Generators use the supplied Random, never Rantly's process-global randomness.
module PolicyPropertySupport
  PROFILE = ENV.fetch("RUBY_PROPERTY_PROFILE", "ci")
  COUNTS = { "ci" => 100, "explore" => 1_000 }.freeze
  raise ArgumentError, "Unknown RUBY_PROPERTY_PROFILE: #{PROFILE.inspect}" unless COUNTS.key?(PROFILE)

  SEED_TEXT = ENV.fetch("RUBY_PROPERTY_SEED", "20260930")
  unless SEED_TEXT.match?(/\A[0-9]+\z/) && SEED_TEXT.to_i < 2**64
    raise ArgumentError, "RUBY_PROPERTY_SEED must be an unsigned 64-bit integer"
  end
  SEED = SEED_TEXT.to_i

  def check_property(name, corpus: [], generate:, describe: ->(sample) { sample.inspect }, &assertion)
    label = "#{self.class.name}/#{name}"
    seed = SEED ^ Zlib.crc32(label)
    random = Random.new(seed)
    property = nil
    corpus.each do |sample|
      property = Rantly::Property.new(proc { Tuple.new(sample) })
      property.check(1) { |value| assertion.call(value.array) }
    end
    property = Rantly::Property.new(proc { Tuple.new(generate.call(random)) })
    property.check(COUNTS.fetch(PROFILE)) { |sample| assertion.call(sample.array) }
  rescue Minitest::Assertion, StandardError => error
    original = property&.failed_data&.array
    reduced = property&.shrunk_failed_data&.array
    detail = "property=#{label} profile=#{PROFILE} seed=#{SEED} derived_seed=#{seed}\n" \
             "original=#{original ? describe.call(original) : 'not available'}\n" \
             "reduced=#{reduced ? describe.call(reduced) : 'not available'}\n#{error.message}"
    raise error, detail, error.backtrace
  end
end
