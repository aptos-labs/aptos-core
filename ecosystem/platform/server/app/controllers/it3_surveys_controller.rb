# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class It3SurveysController < ApplicationController
  before_action :authenticate_user!
  before_action :ensure_confirmed!
  before_action :set_it3_survey, only: %i[show edit update destroy]
  layout 'it3'
  respond_to :html

  def show
    redirect_to edit_it3_survey_path(params.fetch(:id))
  end

  # GET /it3_surveys/new
  def new
    redirect_to edit_it3_survey_path(current_user.it3_survey) if current_user.it3_survey.present?
    @it3_survey = It3Survey.new
  end

  # GET /it3_surveys/1/edit
  def edit; end

  # POST /it3_surveys
  def create
    params = it3_survey_params
    params[:user] = current_user
    @it3_survey = It3Survey.new(params)

    if @it3_survey.save
      log @it3_survey, 'created'
      redirect_to it3_path, notice: 'Your survey response has been recorded.'
    else
      respond_with(@it3_survey, status: :unprocessable_entity)
    end
  end

  # PATCH/PUT /it3_surveys/1
  def update
    if @it3_survey.update(it3_survey_params)
      log @it3_survey, 'updated'
      redirect_to it3_path, notice: 'Survey response updated.'
    else
      respond_with(@it3_survey, status: :unprocessable_entity)
    end
  end

  private

  def set_it3_survey
    @it3_survey = It3Survey.find(params[:id])
    head :forbidden unless @it3_survey.user_id == current_user.id
  end

  def it3_survey_params
    params.require(:it3_survey).permit(:user_id, :persona, :participate_reason, :qualified_reason, :website,
                                       :interest_reason)
  end
end
