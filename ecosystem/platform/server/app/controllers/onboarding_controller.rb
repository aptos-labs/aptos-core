# frozen_string_literal: true

class OnboardingController < ApplicationController
  def email
    @oauth_username = current_user.authorizations.pluck(:username).first
    @oauth_email = current_user.authorizations.pluck(:email).first
  end

  def email_update
    email_params = params.require(:user).permit(:email)
    if current_user.update(email_params)
      current_user.send_confirmation_instructions
      redirect_to overview_index_path
    else
      render :email, status: :unprocessable_entity
    end
  end
end
