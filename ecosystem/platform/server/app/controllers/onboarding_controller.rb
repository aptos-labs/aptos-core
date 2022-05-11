# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class OnboardingController < ApplicationController
  before_action :authenticate_user!
  before_action :set_oauth_data
  layout 'it1'

  def email; end

  def email_update
    return redirect_to overview_index_path if current_user.confirmed?

    email_params = params.require(:user).permit(:email, :username)
    if verify_recaptcha(model: current_user) && current_user.update(email_params)
      log current_user, 'email updated'
      current_user.send_confirmation_instructions
      redirect_to onboarding_email_path, notice: "Verification email sent to #{email_params[:email]}"
    else
      render :email, status: :unprocessable_entity
    end
  end

  private

  def set_oauth_data
    @oauth_username = current_user.authorizations.pluck(:username).first
    @oauth_email = current_user.authorizations.pluck(:email).first
  end
end
