# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class LeaderboardControllerTest < ActionDispatch::IntegrationTest
  test 'loads correctly' do
    get it1_path
    assert_response :success
  end

  test 'loads correctly with sort param' do
    get it1_path(sort: '-participation,liveness')
    assert_response :success
  end

  test 'loads correctly with malformed sort param' do
    get it1_path(sort: { '$foo': 1 })
    assert_response :success
  end
end
