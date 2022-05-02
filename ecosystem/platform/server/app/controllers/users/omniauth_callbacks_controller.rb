# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module Users
  class OmniauthCallbacksController < Devise::OmniauthCallbacksController
    def github
      @user = User.from_omniauth(auth_data, current_user)
      if @user.persisted?
        @message = I18n.t 'devise.omniauth_callbacks.success', kind: 'Github'
        sign_in(@user)
      else
        # TODO: make this bulletproof
        @message = I18n.t 'devise.omniauth_callbacks.failure', kind: :github,
                                                               reason: @user.errors.full_messages.join("\n")
      end

      # TODO: RENDER SOME RESPONSE!
      # render json: { message: }
      render 'api/users/show', formats: [:json]
    end

    def discord
      @user = User.from_omniauth(auth_data, current_user)

      if @user.persisted?
        message = I18n.t 'devise.omniauth_callbacks.success', kind: 'Discord'
        sign_in(@user)
      else
        # TODO: make this bulletproof
        message = I18n.t 'devise.omniauth_callbacks.failure', kind: :discord,
                                                              reason: @user.errors.full_messages.join("\n")
      end

      # TODO: RENDER SOME RESPONSE!
      render json: { message: }
    end

    private

    def auth_data
      request.env['omniauth.auth']
    end
  end
end
