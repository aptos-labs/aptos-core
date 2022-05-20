# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class WelcomeController < ApplicationController
  layout 'it1'

  before_action :ensure_confirmed!, only: %i[it1]

  def index
    @login_dialog = DialogComponent.new
  end

  def it1
    redirect_to root_path unless user_signed_in?
    @steps = [
      connect_discord_step,
      node_registration_step,
      identity_verification_step
    ].map { |h| OpenStruct.new(**h) }
    first_incomplete = @steps.index { |step| step.completed == false }
    @steps[first_incomplete + 1..].each { |step| step.disabled = true } if first_incomplete
  end

  private

  def connect_discord_step
    {
      name: :connect_discord,
      completed: current_user.authorizations.where(provider: :discord).exists?,
      dialog: DialogComponent.new
    }
  end

  def node_registration_step
    completed = !current_user&.it1_profile&.validator_verified?.nil?
    {
      name: :node_registration,
      completed:,
      href: completed ? edit_it1_profile_path(current_user.it1_profile) : new_it1_profile_path
    }
  end

  def identity_verification_step
    completed = !current_user&.kyc_complete?.nil?
    {
      name: :identity_verification,
      completed:,
      href: completed ? nil : onboarding_kyc_redirect_path
    }
  end
end
