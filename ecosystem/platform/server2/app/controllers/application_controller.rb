# frozen_string_literal: true

class ApplicationController < ActionController::Base
  # required for activeadmin
  include ActionView::Layouts

  protect_from_forgery with: :exception

  before_action :configure_permitted_parameters, if: :devise_controller?

  def admin_access_denied(_exception)
    head :forbidden
  end
end
