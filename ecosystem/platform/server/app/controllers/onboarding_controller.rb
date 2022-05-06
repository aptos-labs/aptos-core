# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class OnboardingController < ApplicationController
  before_action :set_oauth_data

  def email; end

  def email_update
    email_params = params.require(:user).permit(:email)
    if current_user.update(email_params)
      current_user.send_confirmation_instructions
      redirect_to overview_index_path
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
