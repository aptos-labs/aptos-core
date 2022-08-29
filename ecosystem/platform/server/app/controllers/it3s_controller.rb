# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It3sController < ApplicationController
  layout 'it3'

  before_action :authenticate_user!
  before_action :ensure_confirmed!
  before_action :ensure_registration_open!

  def show
    @it3_registration_closed = Flipper.enabled?(:it3_registration_closed, current_user)
    @steps = [
      connect_discord_step,
      connect_wallet_step,
      survey_step,
      node_registration_step,
      identity_verification_step
    ].map do |h|
      # rubocop:disable Style/OpenStructUse
      OpenStruct.new(**h)
      # rubocop:enable Style/OpenStructUse
    end
    first_incomplete = @steps.index { |step| !step.completed }
    @steps[first_incomplete + 1..].each { |step| step.disabled = true } if first_incomplete
    @steps.each { |step| step.disabled = true } if @it3_registration_closed
  end

  # Updates the owner key when the wallet is connected.
  def update
    owner_key = params.require(:owner_key)

    if !owner_key.is_a?(String) || !owner_key.match(/\A0x[a-f0-9]{64}\z/i)
      return render plain: 'Invalid request', status: :unprocessable_entity
    end

    session[:it3_owner_key] = owner_key
    redirect_to it3_path
  end

  private

  def ensure_registration_open!
    redirect_to root_path unless Flipper.enabled?(:it3_registration_open)
  end

  def connect_discord_step
    completed = current_user.authorizations.where(provider: :discord).exists?
    {
      name: :connect_discord,
      completed:,
      dialog: completed ? nil : DialogComponent.new
    }
  end

  def connect_wallet_step
    completed = !!(current_user.it3_profile&.owner_key || session[:it3_owner_key])
    {
      name: :connect_wallet,
      completed:,
      dialog: completed ? nil : DialogComponent.new
    }
  end

  def survey_step
    completed = !current_user.it3_survey.nil?
    {
      name: :survey,
      disabled: !Flipper.enabled?(:it3_node_registration_enabled, current_user),
      completed:,
      href: completed ? edit_it3_survey_path(current_user.it3_survey) : new_it3_survey_path
    }
  end

  def node_registration_step
    completed = !!current_user.it3_profile&.validator_verified?
    {
      name: :node_registration,
      completed:,
      disabled: !Flipper.enabled?(:it3_node_registration_enabled, current_user),
      href: completed ? edit_it3_profile_path(current_user.it3_profile) : new_it3_profile_path
    }
  end

  def identity_verification_step
    completed = current_user.kyc_complete?
    {
      name: :identity_verification,
      completed:,
      href: completed ? nil : onboarding_kyc_redirect_path
    }
  end
end
