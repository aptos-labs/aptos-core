# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class LintFactoriesTest < ActiveSupport::TestCase
  test 'all factories can be created' do
    FactoryBot.lint traits: true
  end
end
