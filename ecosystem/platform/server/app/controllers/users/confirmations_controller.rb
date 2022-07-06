# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

module Users
  class ConfirmationsController < Devise::ConfirmationsController
    layout 'it2'

    # GET /resource/confirmation?confirmation_token=abcdef
    def show
      self.resource = resource_class.confirm_by_token(params[:confirmation_token])
      sign_in(resource) if resource.persisted?

      yield resource if block_given?

      # handle trying to confirm twice
      # TODO: reduce token expiry to 1m on successful login
      if resource.persisted? && resource.confirmed? && resource.unconfirmed_email.blank?
        set_flash_message!(:notice, :confirmed)
        if current_user.present?
          redirect_to settings_profile_url, notice: find_message(:confirmed)
        else
          # This branch should never get hit; if token is valid, user is logged in by now
          redirect_to root_path, notice: find_message(:confirmed)
        end
        return
      end

      # Handle whether confirmation worked or not
      if resource.errors.empty?
        set_flash_message!(:notice, :confirmed)
        # If no errors, we're already logged in!
        respond_with_navigational(resource) { redirect_to after_confirmation_path_for(resource_name, resource) }
      else
        respond_with_navigational(resource.errors) do
          if current_user.present?
            redirect_to settings_profile_url, alert: 'Confirmation token is invalid'
          else
            redirect_to root_path, alert: 'Confirmation token is invalid'
          end
        end
      end
    end
  end
end
