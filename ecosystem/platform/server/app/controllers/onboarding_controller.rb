# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class OnboardingController < ApplicationController
  before_action :authenticate_user!, except: %i[kyc_callback]
  before_action :ensure_discord!, only: %i[kyc_redirect]
  before_action :ensure_confirmed!, only: %i[kyc_redirect]
  before_action :ensure_it2_registration_open!, only: %i[kyc_callback kyc_redirect]
  before_action :set_oauth_data, except: %i[kyc_callback]
  protect_from_forgery except: :kyc_callback

  layout 'it2'

  def email
    redirect_to it2_path if current_user.confirmed?
  end

  def email_success; end

  def email_update
    redirect_to it2_path and return if current_user.confirmed?

    recaptcha_v3_success = verify_recaptcha(action: 'onboarding/email', minimum_score: 0.5,
                                            secret_key: ENV.fetch('RECAPTCHA_V3_SECRET_KEY', nil), model: current_user)
    recaptcha_v2_success = verify_recaptcha(model: current_user) unless recaptcha_v3_success
    unless recaptcha_v3_success || recaptcha_v2_success
      @show_recaptcha_v2 = true
      return render :email, status: :unprocessable_entity
    end

    email_params = params.require(:user).permit(:email, :username, :terms_accepted)
    if current_user.update(email_params)
      log current_user, 'email updated'
      if forum_sso?
        redirect_to discourse_sso_path
      else
        redirect_to onboarding_email_success_path
      end
    else
      render :email, status: :unprocessable_entity
    end
  rescue SendEmailJobError
    current_user.errors.add :email
    render :email, status: :unprocessable_entity
  end

  def kyc_redirect
    if current_user.kyc_exempt?
      redirect_to it2_path,
                  notice: 'You are not required to complete Identity Verification' and return
    end
    if current_user.kyc_complete?
      redirect_to it2_path,
                  notice: 'You have already completed Identity Verification' and return
    end

    unless current_user.it2_profile&.validator_verified?
      path = current_user.it2_profile.present? ? edit_it2_profile_path(current_user.it2_profile) : new_it2_profile_path
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

    # we don't have a current user if we're doing personas "complete on another device" thing
    if current_user.present?
      redirect_to onboarding_email_path and return unless current_user.confirmed?
      if current_user.external_id != reference_id
        redirect_to onboarding_kyc_redirect_path,
                    status: :unprocessable_entity, error: 'Persona was started with a different user' and return
      end
    end

    inquiry_id = kyc_params.require(:'inquiry-id')
    begin
      KYCCompleteJob.perform_now({ user_id: current_user&.id, inquiry_id:, external_id: reference_id })
      redirect_to it2_path, notice: 'Identity Verification completed successfully!'
    rescue KYCCompleteJobError => e
      Sentry.capture_exception(e)
      redirect_to it2_path, error: 'Error; If you completed Identity Verification, ' \
                                   "it may take some time to reflect. Error: #{e}"
    end
  end

  private

  def set_oauth_data
    @oauth_username = current_user.authorizations.pluck(:username).first
    @oauth_email = current_user.authorizations.pluck(:email).first
  end

  def ensure_it2_registration_open!
    redirect_to root_url if Flipper.enabled?(:it2_registration_closed, current_user)
  end
end
