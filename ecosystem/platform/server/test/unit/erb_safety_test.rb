# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# frozen_string_literal: true

require 'test_helper'
require 'better_html/test_helper/safe_erb_tester'

class ErbSafetyTest < ActiveSupport::TestCase
  include BetterHtml::TestHelper::SafeErbTester
  ERB_GLOB = Rails.root.join(
    'app', '**', '{*.htm,*.html,*.htm.erb,*.html.erb,*.html+*.erb}'
  )

  Dir[ERB_GLOB].each do |filename|
    pathname = Pathname.new(filename).relative_path_from(Rails.root)
    test "missing javascript escapes in #{pathname}" do
      assert_erb_safety File.read(filename)
    end
  end
end
