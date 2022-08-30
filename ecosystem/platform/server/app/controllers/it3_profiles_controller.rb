# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It3ProfilesController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_registration_enabled!
  before_action :ensure_confirmed!
  before_action :set_it3_profile, only: %i[show edit update destroy]
  respond_to :html

  def show
    redirect_to edit_it3_profile_path(params.fetch(:id))
  end

  # GET /it3_profiles/new
  def new
    redirect_to edit_it3_profile_path(current_user.it3_profile) if current_user.it3_profile.present?
    @it3_profile = It3Profile.new(owner_key: session[:it3_owner_key])
  end

  # GET /it3_profiles/1/edit
  def edit; end

  # POST /it3_profiles or /it3_profiles.json
  def create
    params = it3_profile_params
    params[:user] = current_user
    @it3_profile = It3Profile.new(params)

    return unless check_recaptcha

    v = NodeHelper::NodeVerifier.new(@it3_profile.validator_address,
                                     @it3_profile.validator_metrics_port,
                                     @it3_profile.validator_api_port)

    if v.ip.ok
      @it3_profile.validator_ip = v.ip.ip
    else
      @it3_profile.errors.add :validator_address, v.ip.message
      respond_with(@it3_profile, status: :unprocessable_entity) and return
    end

    if @it3_profile.save
      log @it3_profile, 'created'

      session.delete(:it3_owner_key)

      if Flipper.enabled?(:node_health_checker)
        @it3_profile.enqueue_nhc_job(true)
      else
        validate_node(v, do_location: true)
      end

      if @it3_profile.validator_verified?
        current_user.maybe_send_ait3_registration_complete_email
        redirect_to it3_path, notice: 'AIT3 application completed successfully: your node is verified!' and return
      end
    end
    respond_with(@it3_profile, status: :unprocessable_entity)
  end

  # PATCH/PUT /it3_profiles/1 or /it3_profiles/1.json
  def update
    v = NodeHelper::NodeVerifier.new(it3_profile_params[:validator_address],
                                     it3_profile_params[:validator_metrics_port],
                                     it3_profile_params[:validator_api_port])

    return unless check_recaptcha

    if v.ip.ok
      @it3_profile.validator_ip = v.ip.ip
    else
      @it3_profile.errors.add :validator_address, v.ip.message
      respond_with(@it3_profile, status: :unprocessable_entity) and return
    end

    ip_changed = @it3_profile.validator_ip_changed?
    if @it3_profile.update(it3_profile_params)
      log @it3_profile, 'updated'
      if @it3_profile.validator_verified? && !@it3_profile.needs_revalidation?
        redirect_to it3_path,
                    notice: 'AIT3 node information updated' and return
      end

      if Flipper.enabled?(:node_health_checker)
        @it3_profile.enqueue_nhc_job(ip_changed)
      else
        validate_node(v, do_location: ip_changed)
      end

      if @it3_profile.validator_verified?
        redirect_to it3_path, notice: 'AIT3 node verification completed successfully!' and return
      end
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
    params.fetch(:it3_profile, {}).permit(:owner_key,
                                          :consensus_key, :consensus_pop, :account_key, :network_key,
                                          :validator_address, :validator_port, :validator_api_port,
                                          :validator_metrics_port, :fullnode_address, :fullnode_port,
                                          :fullnode_network_key, :terms_accepted)
  end

  def ensure_registration_enabled!
    return redirect_to root_path unless Flipper.enabled?(:it3_registration_open)
    return redirect_to it3_path unless Flipper.enabled?(:it3_node_registration_enabled, current_user)
    return redirect_to it3_path if Flipper.enabled?(:it3_registration_closed) &&
                                   !Flipper.enabled?(:it3_registration_override, current_user)
  end
end
