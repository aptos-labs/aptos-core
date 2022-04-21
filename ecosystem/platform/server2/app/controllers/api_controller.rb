# frozen_string_literal: true

class ApiController < ActionController::API
  include AbstractController::Translation
  include ActionController::Cookies
end
