# frozen_string_literal: true

require "strscan"

module PrCiPolicy
  # The one lexer for GitHub Actions expressions. GitHub string literals use
  # single quotes, with '' for a literal quote. A double quote outside a string,
  # an unterminated string, or a ${{ without }} raises Unparseable, so callers
  # can fail closed. Every pass over the tokens is linear in their number.
  module Expression
    # `start` and `finish` are byte offsets into the scanned text.
    Token = Data.define(:type, :value, :start, :finish)

    class Unparseable < StandardError; end

    WHITESPACE = /\s+/
    TEMPLATE_START = /\$\{\{/
    TEMPLATE_END = /\}\}/
    IDENTIFIER = /[A-Za-z_][A-Za-z0-9_]*/
    STRING = /'((?:[^']|'')*+)'/
    QUOTE = /['"]/
    OPERATOR = /&&|\|\||==|!=|<=|>=/

    module_function

    # Tokens of a bare expression, such as a job `if:` without ${{ }}.
    def tokenize(text)
      scan(StringScanner.new(text), template: false)
    end

    # Token lists of every ${{ ... }} in `text`, in order.
    def embedded(text)
      scanner = StringScanner.new(text)
      expressions = []
      expressions << scan(scanner, template: true) while scanner.skip_until(TEMPLATE_START)
      expressions
    end

    # Tokens when `text` is exactly one ${{ ... }} template, or nil.
    def whole_template(text)
      scanner = StringScanner.new(text)
      return nil unless scanner.skip(TEMPLATE_START)

      tokens = scan(scanner, template: true)
      tokens if scanner.eos?
    end

    def scan(scanner, template:)
      tokens = []
      until scanner.eos?
        start = scanner.pos
        next if scanner.skip(WHITESPACE)
        return tokens if template && scanner.skip(TEMPLATE_END)

        if scanner.scan(IDENTIFIER)
          tokens << Token.new(type: :identifier, value: scanner.matched, start: start, finish: scanner.pos)
        elsif scanner.scan(STRING)
          tokens << Token.new(type: :string, value: scanner[1].gsub("''", "'"), start: start, finish: scanner.pos)
        elsif scanner.check(QUOTE)
          raise Unparseable, "unsupported or unterminated string at offset #{start}"
        else
          value = scanner.scan(OPERATOR) || scanner.getch
          tokens << Token.new(type: :symbol, value: value, start: start, finish: scanner.pos)
        end
      end
      raise Unparseable, "unterminated ${{ expression" if template

      tokens
    end

    def symbol?(token, value)
      !token.nil? && token.type == :symbol && token.value == value
    end

    # Maps the index of each opening token to the index of its closing token.
    def matching_pairs(tokens, opening, closing)
      open = []
      pairs = {}
      tokens.each_with_index do |token, index|
        if symbol?(token, opening)
          open << index
        elsif symbol?(token, closing) && !open.empty?
          pairs[open.pop] = index
        end
      end
      pairs
    end

    # Removes parentheses that wrap the whole token list, at any depth.
    def strip_parentheses(tokens)
      pairs = matching_pairs(tokens, "(", ")")
      first = 0
      last = tokens.length - 1
      while first < last && pairs[first] == last
        first += 1
        last -= 1
      end
      tokens[first..last]
    end

    # Splits at top-level && operators. Returns nil when the expression has a
    # top-level ||, unbalanced parentheses, or an empty operand.
    def conjuncts(tokens)
      parts = [[]]
      depth = 0
      tokens.each do |token|
        if symbol?(token, "(")
          depth += 1
        elsif symbol?(token, ")")
          depth -= 1
          return nil if depth.negative?
        elsif depth.zero? && symbol?(token, "||")
          return nil
        elsif depth.zero? && symbol?(token, "&&")
          parts << []
          next
        end
        parts.last << token
      end
      parts if depth.zero? && parts.none?(&:empty?)
    end

    # Number of leading tokens that spell the member chain `names` (`a.b` or
    # `a['b']`, case-insensitive on ASCII bytes), or nil. A bracket member is
    # a string literal, so it can hold non-ASCII bytes; Ruby's casecmp? uses
    # full Unicode case folding, which can equate a non-ASCII lookalike (for
    # example 'ſ', U+017F) with an ASCII `name`, while GitHub's own property
    # lookup is ordinal. A bracket member that is not ascii_only? never matches.
    def member_chain_length(tokens, names)
      root, *members = names
      return nil unless tokens[0]&.type == :identifier && tokens[0].value.casecmp?(root)

      index = 1
      members.each do |name|
        if symbol?(tokens[index], ".") && tokens[index + 1]&.type == :identifier && tokens[index + 1].value.casecmp?(name)
          index += 2
        elsif symbol?(tokens[index], "[") && tokens[index + 1]&.type == :string &&
              tokens[index + 1].value.ascii_only? && tokens[index + 1].value.casecmp?(name) && symbol?(tokens[index + 2], "]")
          index += 3
        else
          return nil
        end
      end
      index
    end
  end
end
