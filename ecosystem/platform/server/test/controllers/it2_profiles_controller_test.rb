# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'
require 'minitest/autorun'

class It2ProfilesControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  setup do
    @user = FactoryBot.create(:user)
    sign_in @user
    Flipper.enable(:it2_registration_open)
  end

  test 'new profile page' do
    get new_it2_profile_path
    assert_response :success
  end

  test 'create new profile' do
    assert_nil @user.it2_profile
    post it2_profiles_path, params: { it2_profile: {
      consensus_key: '0xbcaa0d44b821a745bc29767713cd78dbc88da73679e3ccdf5c145a2b4f7b17ac',
      account_key: '0x7964a378e4c6d387d900c6e02430b7ee8263a977ace368484fc72c3b8469f520',
      network_key: '0x2b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b',
      validator_address: '127.0.0.1',
      validator_port: '6180',
      validator_metrics_port: '9101',
      validator_api_port: '8080',
      terms_accepted: '1'
    } }
    assert_not_nil @user.it2_profile
    assert @user.it2_profile.persisted?
    assert_redirected_to it2_profile_path(@user.it2_profile)
  end

  test 'update existing profile' do
    it2_profile = FactoryBot.create(:it2_profile, user: @user)

    patch it2_profile_path(it2_profile), params: { it2_profile: {
      validator_address: '127.0.0.1',
      validator_port: '6180',
      validator_metrics_port: '9101',
      validator_api_port: '8080',
      fullnode_address: '127.0.0.1',
      fullnode_port: '9999',
      fullnode_network_key: '0x1b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b',
      terms_accepted: '1'
    } }

    it2_profile = It2Profile.find(@user.it2_profile.id)
    assert_equal '127.0.0.1', it2_profile.fullnode_address
    assert_equal 9999, it2_profile.fullnode_port
    assert_equal '0x1b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b',
                 it2_profile.fullnode_network_key
  end
end
