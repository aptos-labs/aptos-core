# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class DiscourseControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  setup do
    ENV['DISCOURSE_SECRET'] = 'd836444a9e4084d5b224a60c208dce14'
    user = FactoryBot.create(:user, { email: 'foo@example.com', username: 'foo', confirmed_at: DateTime.now })
    sign_in user
  end

  test 'it redirects to discourse/session/sso if query string is blank' do
    get discourse_sso_url
    assert_redirected_to 'https://forum.aptoslabs.com/session/sso?return_path=%2F'
  end

  test 'it redirects to discourse/session/sso_login if query string contains sso info' do
    get discourse_sso_url(sso: 'bm9uY2U9Y2I2ODI1MWVlZmI1MjExZTU4YzAwZmYxMzk1ZjBjMGI=',
                          sig: '1ce1494f94484b6f6a092be9b15ccc1cdafb1f8460a3838fbb0e0883c4390471')
    assert_redirected_to 'https://forum.aptoslabs.com/session/sso_login?sso=YWRtaW49ZmFsc2UmZW1haWw9Zm9vJTQwZXhhbXBsZS5jb20mbm9uY2U9Y2I2ODI1MWVlZmI1MjExZTU4YzAwZmYxMzk1ZjBjMGImdXNlcm5hbWU9Zm9v&sig=5e5c68081a49d907054f060f9acd5fa18950d6b6a4b00601ca6258987840dc2d'
  end
end
