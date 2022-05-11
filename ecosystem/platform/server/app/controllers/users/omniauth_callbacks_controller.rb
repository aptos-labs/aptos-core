# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module Users
  class OmniauthCallbacksController < Devise::OmniauthCallbacksController
    User.omniauth_providers.each do |provider|
      define_method provider do
        oauth_callback(provider)
      end
    end

    private

    def oauth_callback(provider)
      @user = User.from_omniauth(auth_data, current_user)

      # TODO: make this bulletproof
      raise 'Unable to persist user' unless @user.persisted?

      sign_in_and_redirect @user
      set_flash_message(:notice, :success, kind: provider.to_s.titleize) if is_navigational_format?
    end

    def auth_data
      request.env['omniauth.auth']
    end
  end
end
