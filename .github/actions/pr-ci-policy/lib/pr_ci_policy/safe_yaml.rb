# frozen_string_literal: true

require "psych"
require "set"

module PrCiPolicy
  # Loads workflow YAML as plain data. Object tags, aliases, anchors, merge keys,
  # duplicate keys, and mapping keys that do not load as strings are rejected
  # before loading.
  module SafeYaml
    YAML_11_BOOLEAN_KEYS = Set.new(%w[y yes true on n no false off]).freeze
    SAFE_TAGS = Set.new(%w[
      tag:yaml.org,2002:null
      tag:yaml.org,2002:bool
      tag:yaml.org,2002:int
      tag:yaml.org,2002:float
      tag:yaml.org,2002:str
      tag:yaml.org,2002:seq
      tag:yaml.org,2002:map
    ]).freeze
    KEY_LOADER = Psych::ClassLoader::Restricted.new([], [])

    module_function

    def load(text, source)
      raise PolicyError, "#{source}: workflow content is not text" unless text.is_a?(String)
      raise PolicyError, "#{source}: workflow is not valid UTF-8" unless text.dup.force_encoding(Encoding::UTF_8).valid_encoding?

      stream = Psych.parse_stream(text, filename: source)
      documents = stream.children
      raise PolicyError, "#{source}: YAML must contain exactly one document" unless documents.length == 1
      validate_node(documents.first.root, source, root_mapping: true)
      data = Psych.safe_load(
        text,
        filename: source,
        permitted_classes: [],
        permitted_symbols: [],
        aliases: false,
      )
      raise PolicyError, "#{source}: workflow root must be a mapping" unless data.is_a?(Hash)

      # Psych uses YAML 1.1 and parses the unquoted GitHub Actions `on` key as
      # boolean true. Normalize only this schema-defined root key.
      data["on"] = data.delete(true) if data.key?(true) && !data.key?("on")
      data
    rescue Psych::Exception => e
      raise PolicyError, "#{source}: unsafe or malformed YAML: #{e.message}"
    end

    def validate_node(node, source, root_mapping: false)
      if node.is_a?(Psych::Nodes::Alias)
        raise PolicyError, "#{source}: YAML aliases are not allowed"
      end
      if node.respond_to?(:anchor) && node.anchor
        raise PolicyError, "#{source}: YAML anchors are not allowed"
      end
      if node.respond_to?(:tag) && node.tag && !SAFE_TAGS.include?(node.tag)
        raise PolicyError, "#{source}: YAML tag #{node.tag.inspect} is not allowed"
      end

      if node.is_a?(Psych::Nodes::Mapping)
        seen = Set.new
        node.children.each_slice(2) do |key, _value|
          validate_key(key, source, root_mapping)
          raise PolicyError, "#{source}: duplicate YAML key #{key.value.inspect}" unless seen.add?(key.value)
        end
      end

      Array(node.children).each { |child| validate_node(child, source, root_mapping: false) } if node.respond_to?(:children)
    end

    def validate_key(key, source, root_mapping)
      raise PolicyError, "#{source}: mapping keys must be scalar strings" unless key.is_a?(Psych::Nodes::Scalar)
      raise PolicyError, "#{source}: YAML merge keys are not allowed" if key.value == "<<"
      return if root_mapping && key.value == "on"
      if YAML_11_BOOLEAN_KEYS.include?(key.value.downcase)
        raise PolicyError, "#{source}: ambiguous YAML boolean key #{key.value.inspect} is not allowed"
      end

      # Resolve the key exactly as Psych.safe_load will, with no permitted classes.
      value = Psych::Visitors::ToRuby.new(Psych::ScalarScanner.new(KEY_LOADER), KEY_LOADER).accept(key)
      raise PolicyError, "#{source}: mapping key #{value.inspect} is not a string" unless value.is_a?(String)
    end
  end
end
