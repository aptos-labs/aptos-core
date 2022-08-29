# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class SettingsControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  setup do
    OmniAuth.config.test_mode = true
    OmniAuth.config.add_mock :github, Faker::Omniauth.github
    OmniAuth.config.add_mock :google, Faker::Omniauth.google
    post user_github_omniauth_authorize_url
    follow_redirect!
    @user = User.last
    sign_in @user
    assert_equal 1, @user.authorizations.count
  end

  test 'profile settings page' do
    get settings_profile_url
    assert_response :success
  end

  test 'update username' do
    patch settings_profile_url(@user), params: { user: { username: 'asdf' } }
    assert_equal 'asdf', @user.username
  end

  test 'update email' do
    patch settings_profile_url(@user), params: { user: { email: 'foobar@example.org' } }
    assert_equal 'foobar@example.org', @user.unconfirmed_email
  end

  test 'notification settings page' do
    get settings_notifications_path
    assert_response :success
  end

  test 'update notification settings' do
    patch settings_notifications_path(@user),
          params: { notification_preference: { node_upgrade_notification: true,
                                               governance_proposal_notification: false } }

    prefs = @user.notification_preferences.where(delivery_method: :email).first
    assert prefs.node_upgrade_notification
    refute prefs.governance_proposal_notification
  end

  test 'connections settings page' do
    get settings_connections_url
    assert_response :success
  end

  test 'remove connection' do
    post user_google_omniauth_authorize_url
    follow_redirect!
    assert_equal 2, @user.authorizations.count
    authorization = @user.authorizations.first
    delete settings_connections_url(authorization), params: { authorization: { id: authorization.id } }
    assert_equal 1, @user.authorizations.count
  end

  test 'remove last connection fails' do
    assert_equal 1, @user.authorizations.count
    authorization = @user.authorizations.first
    delete settings_connections_url(authorization), params: { authorization: { id: authorization.id } }
    assert_equal 1, @user.authorizations.count
  end

  test 'deletes account successfully' do
    @user.it3_profile = FactoryBot.create(:it3_profile, user: @user)
    @user.it3_survey = FactoryBot.create(:it3_survey, user: @user)
    delete settings_delete_account_url,
           params: { user: { verification_text: 'delete my account 55555', verification_number: 55_555 } }
    follow_redirect!
    refute User.where(id: @user.id).exists?
  end

  test 'delete account enforces verification' do
    delete settings_delete_account_url,
           params: { user: { verification_text: 'delete my account 55555', verification_number: 333 } }
    assert_response 422
    assert User.where(id: @user.id).exists?
  end
end
