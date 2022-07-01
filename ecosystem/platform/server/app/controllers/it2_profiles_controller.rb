# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It2ProfilesController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_registration_enabled!
  before_action :ensure_confirmed!
  before_action :set_it2_profile, only: %i[show edit update destroy]
  respond_to :html

  def show
    redirect_to edit_it2_profile_path(params.fetch(:id))
  end

  # GET /it2_profiles/new
  def new
    redirect_to edit_it2_profile_path(current_user.it2_profile) if current_user.it2_profile.present?
    @it2_profile = It2Profile.new
  end

  # GET /it2_profiles/1/edit
  def edit; end

  # POST /it2_profiles or /it2_profiles.json
  def create
    params = it2_profile_params
    params[:user] = current_user
    @it2_profile = It2Profile.new(params)

    return unless check_recaptcha

    v = NodeHelper::NodeVerifier.new(@it2_profile.validator_address,
                                     @it2_profile.validator_metrics_port,
                                     @it2_profile.validator_api_port)

    if v.ip.ok
      @it2_profile.validator_ip = v.ip.ip
    else
      @it2_profile.errors.add :validator_address, v.ip.message
      respond_with(@it2_profile, status: :unprocessable_entity) and return
    end

    if @it2_profile.save
      log @it2_profile, 'created'

      if Flipper.enabled?(:node_health_checker)
        @it2_profile.enqueue_nhc_job(true)
      else
        validate_node(v, do_location: true)
      end

      if @it2_profile.validator_verified?
        current_user.maybe_send_ait2_registration_complete_email
        redirect_to it2_path, notice: 'AIT2 application completed successfully: your node is verified!' and return
      end
    end
    respond_with(@it2_profile, status: :unprocessable_entity)
  end

  # PATCH/PUT /it2_profiles/1 or /it2_profiles/1.json
  def update
    v = NodeHelper::NodeVerifier.new(it2_profile_params[:validator_address],
                                     it2_profile_params[:validator_metrics_port],
                                     it2_profile_params[:validator_api_port])

    return unless check_recaptcha

    if v.ip.ok
      @it2_profile.validator_ip = v.ip.ip
    else
      @it2_profile.errors.add :validator_address, v.ip.message
      respond_with(@it2_profile, status: :unprocessable_entity) and return
    end

    ip_changed = @it2_profile.validator_ip_changed?
    if @it2_profile.update(it2_profile_params)
      log @it2_profile, 'updated'
      if @it2_profile.validator_verified? && !@it2_profile.needs_revalidation?
        redirect_to it2_path,
                    notice: 'AIT2 node information updated' and return
      end

      if Flipper.enabled?(:node_health_checker)
        @it2_profile.enqueue_nhc_job(ip_changed)
      else
        validate_node(v, do_location: ip_changed)
      end

      if @it2_profile.validator_verified?
        redirect_to it2_path, notice: 'AIT2 node verification completed successfully!' and return
      end
    end
    respond_with(@it2_profile, status: :unprocessable_entity)
  end

  private

  # @param [NodeHelper::NodeVerifier] node_verifier
  # @return [Array<VerifyResult>]
  def validate_node(node_verifier, do_location: false)
    results = node_verifier.verify

    # Save without validation to avoid needless uniqueness checks
    is_valid = results.map(&:valid).all?
    @it2_profile.update_attribute(:validator_verified, is_valid)

    LocationJob.perform_later({ it2_profile_id: @it2_profile.id }) if is_valid && do_location

    results.each do |result|
      @it2_profile.errors.add :base, result.message unless result.valid
    end
    results
  end

  def validate_node_nhc(node_verifier, do_location: false)
    results = node_verifier.verify(ENV.fetch('NODE_CHECKER_BASELINE_CONFIG'))

    unless results.ok
      @it2_profile.update_attribute(:validator_verified, false)
      @it2_profile.errors.add :base, results.message
      return results
    end

    # Save without validation to avoid needless uniqueness checks
    is_valid = results.evaluation_results.map { |r| r.score == 100 }.all?
    @it2_profile.update_attribute(:validator_verified, is_valid)

    LocationJob.perform_later({ it2_profile_id: @it2_profile.id }) if is_valid && do_location

    results.evaluation_results.each do |result|
      next unless result.score < 100

      message = "#{result.category}: #{result.evaluator_name} - #{result.score}\n" \
                "#{result.headline}:\n" \
                "#{result.explanation}\n" \
                "#{result.links}\n"
      @it2_profile.errors.add :base, message
    end
    results
  end

  def check_recaptcha
    recaptcha_v3_success = verify_recaptcha(action: 'it2/update', minimum_score: 0.5,
                                            secret_key: ENV.fetch('RECAPTCHA_V3_SECRET_KEY', nil), model: @it2_profile)
    recaptcha_v2_success = verify_recaptcha(model: @it2_profile) unless recaptcha_v3_success
    unless recaptcha_v3_success || recaptcha_v2_success
      @show_recaptcha_v2 = true
      respond_with(@it2_profile, status: :unprocessable_entity)
      return false
    end
    true
  end

  # Use callbacks to share common setup or constraints between actions.
  def set_it2_profile
    @it2_profile = It2Profile.find(params[:id])
    head :forbidden unless @it2_profile.user_id == current_user.id
  end

  # Only allow a list of trusted parameters through.
  def it2_profile_params
    params.fetch(:it2_profile, {}).permit(:consensus_key, :account_key, :network_key, :validator_address,
                                          :validator_port, :validator_api_port, :validator_metrics_port,
                                          :fullnode_address, :fullnode_port, :fullnode_network_key, :terms_accepted)
  end

  def ensure_registration_enabled!
    redirect_to root_path unless Flipper.enabled?(:it2_registration_open)
    redirect_to it2_path if Flipper.enabled?(:it2_node_registration_disabled,
                                             current_user) || Flipper.enabled?(:it2_registration_closed, current_user)
  end
end
