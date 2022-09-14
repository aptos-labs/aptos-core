# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It3ProfilesController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_registration_enabled!
  before_action :ensure_confirmed!
  before_action :set_it3_profile, only: %i[show edit update destroy]
  respond_to :html

  # GET /it3_profiles/1/edit
  def edit; end

  # PATCH/PUT /it3_profiles/1 or /it3_profiles/1.json
  def update
    return unless check_recaptcha

    if @it3_profile.update(it3_profile_params)
      log @it3_profile, 'updated'
      redirect_to it3_path,
                  notice: 'AIT3 node information updated' and return
    end

    respond_with(@it3_profile, status: :unprocessable_entity)
  end

  private

  # @param [NodeHelper::NodeVerifier] node_verifier
  # @return [Array<VerifyResult>]
  def validate_node(node_verifier, do_location: false)
    results = node_verifier.verify

    # Save without validation to avoid needless uniqueness checks
    is_valid = results.map(&:valid).all?
    @it3_profile.update_attribute(:validator_verified, is_valid)

    LocationJob.perform_later({ it3_profile_id: @it3_profile.id }) if is_valid && do_location

    results.each do |result|
      @it3_profile.errors.add :base, result.message unless result.valid
    end
    results
  end

  def validate_node_nhc(node_verifier, do_location: false)
    results = node_verifier.verify(ENV.fetch('NODE_CHECKER_BASELINE_CONFIG'))

    unless results.ok
      @it3_profile.update_attribute(:validator_verified, false)
      @it3_profile.errors.add :base, results.message
      return results
    end

    # Save without validation to avoid needless uniqueness checks
    is_valid = results.evaluation_results.map { |r| r.score == 100 }.all?
    @it3_profile.update_attribute(:validator_verified, is_valid)

    LocationJob.perform_later({ it3_profile_id: @it3_profile.id }) if is_valid && do_location

    results.evaluation_results.each do |result|
      next unless result.score < 100

      message = "#{result.category}: #{result.evaluator_name} - #{result.score}\n" \
                "#{result.headline}:\n" \
                "#{result.explanation}\n" \
                "#{result.links}\n"
      @it3_profile.errors.add :base, message
    end
    results
  end

  def check_recaptcha
    recaptcha_v3_success = verify_recaptcha(action: 'it3/update', minimum_score: 0.5,
                                            secret_key: ENV.fetch('RECAPTCHA_V3_SECRET_KEY', nil), model: @it3_profile)
    recaptcha_v2_success = verify_recaptcha(model: @it3_profile) unless recaptcha_v3_success
    unless recaptcha_v3_success || recaptcha_v2_success
      @show_recaptcha_v2 = true
      respond_with(@it3_profile, status: :unprocessable_entity)
      return false
    end
    true
  end

  # Use callbacks to share common setup or constraints between actions.
  def set_it3_profile
    @it3_profile = It3Profile.find(params[:id])
    head :forbidden unless @it3_profile.user_id == current_user.id
  end

  # Only allow a list of trusted parameters through.
  def it3_profile_params
    params.fetch(:it3_profile, {}).permit(:fullnode_address, :fullnode_port, :fullnode_metrics_port, :fullnode_api_port,
                                          :fullnode_network_key, :terms_accepted)
  end

  def ensure_registration_enabled!
    return redirect_to root_path unless Flipper.enabled?(:it3_registration_open)
    return redirect_to it3_path unless Flipper.enabled?(:it3_node_registration_enabled, current_user)
    return redirect_to it3_path if Flipper.enabled?(:it3_registration_closed) &&
                                   !Flipper.enabled?(:it3_registration_override, current_user)
  end
end
