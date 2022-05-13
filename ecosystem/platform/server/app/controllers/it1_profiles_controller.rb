# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
# frozen_string_literal: true

class It1ProfilesController < ApplicationController
  before_action :authenticate_user!
  before_action :set_it1_profile, only: %i[show edit update destroy]
  before_action :ensure_confirmed!
  respond_to :html

  def show
    redirect_to edit_it1_profile_path(params.fetch(:id))
  end

  # GET /it1_profiles/new
  def new
    redirect_to edit_it1_profile_path(current_user.it1_profile) if current_user.it1_profile.present?
    @it1_profile = It1Profile.new
  end

  # GET /it1_profiles/1/edit
  def edit; end

  # POST /it1_profiles or /it1_profiles.json
  def create
    params = it1_profile_params
    params[:user] = current_user
    @it1_profile = It1Profile.new(params)

    respond_with(@it1_profile) and return unless verify_recaptcha(model: @it1_profile)

    v = NodeHelper::NodeVerifier.new(@it1_profile.validator_address, @it1_profile.validator_metrics_port,
                                     @it1_profile.validator_api_port)

    if v.ip.ok
      @it1_profile.validator_ip = v.ip.ip
    else
      @it1_profile.errors.add :validator_address, v.ip.message
      respond_with(@it1_profile) and return
    end

    if @it1_profile.save
      log @it1_profile, 'created'
      validate_node(v, do_location: true)
      if @it1_profile.validator_verified?
        current_user.maybe_send_ait1_registration_complete_email
        redirect_to it1_path, notice: 'AIT1 application completed successfully: your node is verified!' and return
      end
    end
    respond_with(@it1_profile)
  end

  # PATCH/PUT /it1_profiles/1 or /it1_profiles/1.json
  def update
    v = NodeHelper::NodeVerifier.new(it1_profile_params[:validator_address],
                                     it1_profile_params[:validator_metrics_port],
                                     it1_profile_params[:validator_api_port])

    respond_with(@it1_profile) and return unless verify_recaptcha(model: @it1_profile)

    if v.ip.ok
      @it1_profile.validator_ip = v.ip.ip
    else
      @it1_profile.errors.add :validator_address, v.ip.message
      respond_with(@it1_profile) and return
    end

    ip_changed = @it1_profile.validator_ip_changed?
    if @it1_profile.update(it1_profile_params)
      log @it1_profile, 'updated'
      if @it1_profile.validator_verified? && !@it1_profile.needs_revalidation?
        redirect_to it1_path,
                    notice: 'AIT1 node information updated' and return
      end

      validate_node(v, do_location: ip_changed)
      if @it1_profile.validator_verified?
        redirect_to it1_path, notice: 'AIT1 node verification completed successfully!' and return
      end
    end
    respond_with(@it1_profile)
  end

  private

  # @param [NodeHelper::NodeVerifier] node_verifier
  # @return [Array<VerifyResult>]
  def validate_node(node_verifier, do_location: false)
    results = node_verifier.verify

    # Save without validation to avoid needless uniqueness checks
    is_valid = results.map(&:valid).all?
    @it1_profile.update_attribute(:validator_verified, is_valid)

    LocationJob.perform_later({ it1_profile_id: @it1_profile.id }) if is_valid && do_location

    results.each do |result|
      @it1_profile.errors.add :base, result.message unless result.valid
    end
    results
  end

  # Use callbacks to share common setup or constraints between actions.
  def set_it1_profile
    @it1_profile = It1Profile.find(params[:id])
    head :forbidden unless @it1_profile.user_id == current_user.id
  end

  # Only allow a list of trusted parameters through.
  def it1_profile_params
    params.fetch(:it1_profile, {}).permit(:consensus_key, :account_key, :network_key, :validator_address,
                                          :validator_port, :validator_api_port, :validator_metrics_port,
                                          :fullnode_address, :fullnode_port, :fullnode_network_key, :terms_accepted)
  end
end
