# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'
require 'mocha/minitest'

class It3ProfilesControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  setup do
    @user = FactoryBot.create(:user)
    sign_in @user
    Flipper.enable(:it3_registration_open)
    Flipper.enable(:it3_node_registration_enabled)
    Flipper.enable(:it3_registration_closed)
    Flipper.enable(:it3_registration_override, @user)
    It3ProfilesController.any_instance.stubs(:verify_recaptcha).returns(true)
    It3ProfilesController.any_instance.stubs(:validate_node).returns([])
  end

  test 'edit profile page' do
    it3_profile = FactoryBot.create(:it3_profile, user: @user)
    get edit_it3_profile_path(it3_profile)
    assert_response :success
  end

  test 'update existing profile' do
    it3_profile = FactoryBot.create(:it3_profile, user: @user)

    patch it3_profile_path(it3_profile), params: { it3_profile: {
      fullnode_address: '127.0.0.3',
      fullnode_network_key: '0x7964a378e4c6d387d900c6e02430b7ee8263a977ace368484fc72c3b8469f520',
      fullnode_port: '6183',
      fullnode_metrics_port: '8101',
      fullnode_api_port: '8081',
      terms_accepted: '1'
    } }

    it3_profile = It3Profile.find(@user.it3_profile.id)
    assert_equal '127.0.0.3', it3_profile.fullnode_address
    assert_equal 6183, it3_profile.fullnode_port
    assert_equal '0x7964a378e4c6d387d900c6e02430b7ee8263a977ace368484fc72c3b8469f520', it3_profile.fullnode_network_key
  end
end
