# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

module Users
  class OmniauthCallbacksControllerTest < ActionDispatch::IntegrationTest
    TEST_NAME = 'Satoshi Nakamoto'
    TEST_EMAIL = 'satoshi@example.com'
    TEST_UID = '123456789'

    setup do
      OmniAuth.config.test_mode = true
      OmniAuth.config.add_mock :github, Faker::Omniauth.github(name: TEST_NAME, email: TEST_EMAIL, uid: TEST_UID)
      OmniAuth.config.add_mock :google, Faker::Omniauth.google(name: TEST_NAME, email: TEST_EMAIL, uid: TEST_UID)

      # Faker doesn't have discord support yet.
      OmniAuth.config.add_mock :discord, {
        'provider' => 'discord',
        'uid' => TEST_UID,
        'info' => { 'name' => TEST_NAME, 'email' => TEST_EMAIL,
                    'image' => "https://cdn.discordapp.com/avatars/#{TEST_UID}/" },
        'credentials' => { 'token' => Faker::Crypto.md5, 'refresh_token' => Faker::Crypto.md5,
                           'expires_at' => Faker::Time.forward.to_i, 'expires' => true },
        'extra' =>
        { 'raw_info' =>
         { 'id' => TEST_UID,
           'username' => 'satoshi',
           'avatar' => nil,
           'avatar_decoration' => nil,
           'discriminator' => '1337',
           'public_flags' => 0,
           'flags' => 0,
           'banner' => nil,
           'banner_color' => nil,
           'accent_color' => nil,
           'locale' => 'en-US',
           'mfa_enabled' => false,
           'email' => TEST_EMAIL,
           'verified' => true } }
      }
    end

    User.omniauth_providers.each do |provider|
      test "new user via #{provider}" do
        authorize_url = public_send("user_#{provider}_omniauth_authorize_url")
        callback_url = public_send("user_#{provider}_omniauth_callback_url")

        # In test mode, authorize immediately redirects to callback.
        post authorize_url
        assert_redirected_to callback_url

        assert_difference('User.count') do
          follow_redirect!
          assert_redirected_to onboarding_email_url
        end

        user = User.last
        auth = Authorization.last
        assert_equal user, auth.user
        assert_equal provider.to_s, auth.provider
        assert_equal TEST_UID, auth.uid
        assert_not_empty auth.token
        assert_equal TEST_EMAIL, auth.email
        assert_equal TEST_NAME, auth.full_name
        assert_match(/^https?:.+/, auth.profile_url)
        assert_not_empty auth.username unless auth.username.nil?
      end
    end
  end
end
