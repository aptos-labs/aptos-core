# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class OnboardingController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_confirmed!, only: %i[kyc_redirect kyc_callback]
  before_action :set_oauth_data, except: :kyc_callback
  protect_from_forgery except: :kyc_callback

  layout 'it1'

  def email
    redirect_to it1_path if current_user.confirmed?
  end

  def kyc_redirect
    if current_user.kyc_exempt?
      redirect_to it1_path,
                  notice: 'You are not required to complete Identity Verification' and return
    end
    if current_user.kyc_complete?
      redirect_to it1_path,
                  notice: 'You have already completed Identity Verification' and return
    end

    unless current_user.it1_profile&.validator_verified?
      path = current_user.it1_profile.present? ? edit_it1_profile_path(current_user.it1_profile) : new_it1_profile_path
      redirect_to path, error: 'Must register and validate node first' and return
    end

    path = PersonaHelper::PersonaInvite.new(current_user)
                                       .url
                                       .set_param('redirect-uri', onboarding_kyc_callback_url)
                                       .to_s
    redirect_to path, allow_other_host: true
  end

  def kyc_callback
    # inquiry-id=inq_sVMEAhz6fyAHBkmJsMa3hRdw&reference-id=ecbf9114-3539-4bb6-934e-4e84847950e0
    kyc_params = params.permit(:'inquiry-id', :'reference-id')
    reference_id = kyc_params.require(:'reference-id')
    if current_user.external_id != reference_id
      redirect_to onboarding_kyc_redirect_path,
                  status: :unprocessable_entity, error: 'Persona was started with a different user' and return
    end

    inquiry_id = kyc_params.require(:'inquiry-id')
    begin
      KYCCompleteJob.perform_now({ user_id: current_user.id, inquiry_id: })
      redirect_to it1_path, notice: 'Identity Verification completed successfully!'
    rescue KYCCompleteJobError => e
      Sentry.capture_exception(e)
      redirect_to it1_path, error: 'Error; If you completed Identity Verification,'\
                                   " it may take some time to reflect. Error: #{e}"
    end
  end

  def email_update
    redirect_to it1_path and return if current_user.confirmed?
    render :email, status: :unprocessable_entity and return unless verify_recaptcha(model: current_user)

    email_params = params.require(:user).permit(:email, :username)
    if current_user.update(email_params.merge(confirmation_token: Devise.friendly_token))
      log current_user, 'email updated'
      url = confirmation_url(current_user, confirmation_token: current_user.confirmation_token)
      SendConfirmEmailJob.perform_now({ user_id: current_user.id, template_vars: { CONFIRM_LINK: url } })
      render :email_success
    else
      render :email, status: :unprocessable_entity
    end
  rescue SendEmailJobError
    current_user.errors.add :email
    render :email, status: :unprocessable_entity
  end

  private

  def set_oauth_data
    @oauth_username = current_user.authorizations.pluck(:username).first
    @oauth_email = current_user.authorizations.pluck(:email).first
  end
end
