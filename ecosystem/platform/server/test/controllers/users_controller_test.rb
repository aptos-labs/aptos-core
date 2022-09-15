# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class UsersControllerTest < ActionDispatch::IntegrationTest
  setup do
    @user = FactoryBot.create(:user)
    Flipper.enable(:profiles)
  end

  test 'overview loads correctly' do
    get user_url(@user)
    assert_response :success
  end

  test 'projects loads correctly' do
    get user_projects_url(@user)
    assert_response :success
  end

  test 'activity loads correctly' do
    get user_activity_url(@user)
    assert_response :success
  end

  test 'rewards loads correctly' do
    get user_rewards_url(@user)
    assert_response :success
  end
end
