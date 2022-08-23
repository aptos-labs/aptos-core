# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class It3sControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  setup do
    Flipper.enable(:it3_registration_open)
    @user = FactoryBot.create(:user)
    sign_in @user
  end

  teardown do
    sign_out @user
  end

  test 'it loads correctly' do
    get it3_path
    assert_response :success
  end

  test 'if logged out, it redirects to sign in, then redirects back to /it3' do
    sign_out @user
    get it3_path
    assert_redirected_to new_user_session_path

    sign_in @user
    post user_session_path

    assert_redirected_to it3_path
  end
end
