# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class OnboardingControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  test 'onboarding/email page' do
    user = FactoryBot.create(:user, { username: nil, email: nil, confirmed_at: nil })
    sign_in user
    get onboarding_email_path
    assert_response :success
  end

  test 'set username & email' do
    user = FactoryBot.create(:user, { username: nil, email: nil, confirmed_at: nil })
    sign_in user
    post onboarding_email_path, params: { user: { username: 'satoshi', email: 'foobar@example.org' } }
    assert_equal 'satoshi', user.username
    assert_equal 'foobar@example.org', user.unconfirmed_email
    assert_redirected_to onboarding_email_success_path
  end

  test 'set username with already confirmed email' do
    user = FactoryBot.create(:user, { username: nil, email: 'foobar@example.org', confirmed_at: DateTime.now })
    sign_in user
    post onboarding_email_path, params: { user: { username: 'nakamoto' } }
    assert_equal 'nakamoto', user.username
    assert_redirected_to community_path
  end
end
