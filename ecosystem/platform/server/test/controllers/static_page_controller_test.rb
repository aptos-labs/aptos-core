# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

require 'test_helper'

class StaticPageControllerTest < ActionDispatch::IntegrationTest
  include Devise::Test::IntegrationHelpers

  ROUTES = Rails.application.routes.routes.collect do |route|
    ActionDispatch::Routing::RouteWrapper.new route
  end.reject(&:internal?)

  ROUTES.select { |route| route.controller == 'static_page' }.each do |route|
    test "static_page##{route.action} renders ok" do
      sign_out @controller.current_user if @controller&.current_user
      get route.format({})
      assert_response :success

      user = FactoryBot.create(:user)
      sign_in user
      get route.format({})
      assert_response :success
    end
  end
end
