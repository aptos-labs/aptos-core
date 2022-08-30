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

  test 'new profile page' do
    skip('it3 registration closed')
    get new_it3_profile_path
    assert_response :success
  end

  test 'create new profile' do
    skip('it3 registration closed')
    assert_nil @user.it3_profile
    post it3_profiles_path, params: { it3_profile: {
      owner_key: '0xecaa0d44b821a745bc29767713cd78dbc88da73679e3ccdf5c145a2b4f7b17ac',
      consensus_key: "0x#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}"[0...98],
      consensus_pop: "0x#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}#{Faker::Crypto.sha256}"[0...194],
      account_key: '0x7964a378e4c6d387d900c6e02430b7ee8263a977ace368484fc72c3b8469f520',
      network_key: '0x2b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b',
      validator_address: '127.0.0.1',
      validator_port: '6180',
      validator_metrics_port: '9101',
      validator_api_port: '8080',
      terms_accepted: '1'
    } }
    assert_not_nil @user.it3_profile
    assert @user.it3_profile.persisted?
    assert_redirected_to it3_profile_path(@user.it3_profile)
  end

  test 'update existing profile' do
    skip('it3 registration closed')
    it3_profile = FactoryBot.create(:it3_profile, user: @user)

    patch it3_profile_path(it3_profile), params: { it3_profile: {
      validator_address: '127.0.0.1',
      validator_port: '6180',
      validator_metrics_port: '9101',
      validator_api_port: '8080',
      fullnode_address: '127.0.0.1',
      fullnode_port: '9999',
      fullnode_network_key: '0x1b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b',
      terms_accepted: '1'
    } }

    it3_profile = It3Profile.find(@user.it3_profile.id)
    assert_equal '127.0.0.1', it3_profile.fullnode_address
    assert_equal 9999, it3_profile.fullnode_port
    assert_equal '0x1b0ebca9776bd79dcd3c0551e784965e87e8a1551d52c4a48758e1df2122064b',
                 it3_profile.fullnode_network_key
  end
end
