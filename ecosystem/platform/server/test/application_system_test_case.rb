# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class ApplicationSystemTestCase < ActionDispatch::SystemTestCase
  driven_by :selenium, using: :chrome, screen_size: [1400, 1400]
end
