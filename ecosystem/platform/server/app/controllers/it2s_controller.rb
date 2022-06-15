# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It2sController < ApplicationController
  layout 'it2'

  before_action :ensure_confirmed!

  def show
    redirect_to root_path unless user_signed_in?
    redirect_to root_path unless Flipper.enabled?(:it2_registration_open)
    @it2_registration_closed = Flipper.enabled?(:it2_registration_closed, current_user)
    @steps = [
      connect_discord_step,
      node_registration_step,
      identity_verification_step
    ].map { |h| OpenStruct.new(**h) }
    first_incomplete = @steps.index { |step| !step.completed }
    @steps[first_incomplete + 1..].each { |step| step.disabled = true } if first_incomplete
    @steps.each { |step| step.disabled = true } if @it2_registration_closed
  end

  private

  def connect_discord_step
    completed = current_user.authorizations.where(provider: :discord).exists?
    {
      name: :connect_discord,
      completed:,
      dialog: completed ? nil : DialogComponent.new
    }
  end

  def node_registration_step
    completed = !!current_user.it2_profile&.validator_verified?
    {
      name: :node_registration,
      completed:,
      disabled: Flipper.enabled?(:it2_node_registration_disabled, current_user),
      href: completed ? edit_it2_profile_path(current_user.it2_profile) : new_it2_profile_path
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
