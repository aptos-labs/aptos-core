# frozen_string_literal: true

module Api
  class OmniauthCallbacksController < Devise::OmniauthCallbacksController
    def github
      @user = User.from_omniauth(auth_data, current_user)

      if @user.persisted?
        message = I18n.t 'devise.omniauth_callbacks.success', kind: 'Github'
        sign_in(@user)
      else
        # TODO: make this bulletproof
        # We couldn't save the user for some reason (i.e. need to add a username)
        # Removing extra as it can overflow some session stores
        data = auth_data.except('extra')
        # So data will be available after this request when creating the user
        session['devise.oauth.data'] = data
        message = I18n.t 'devise.omniauth_callbacks.failure', reason: @user.errors.full_messages.join("\n")
      end
      # TODO: RENDER SOME RESPONSE!
      render_response(message)
    end

    private

    def render_response(message)
      # @user
      message
    end

    def auth_data
      request.env['omniauth.auth']
    end
  end
end
