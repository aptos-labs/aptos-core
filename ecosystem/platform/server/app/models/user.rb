# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

class User < ApplicationRecord
  # Include devise modules. Others available are:
  # :lockable, :timeoutable, :recoverable,
  devise :database_authenticatable, :confirmable,
         :rememberable, :trackable, :validatable,
         :omniauthable, omniauth_providers: %i[discord github google],
                        authentication_keys: [:username]

  USERNAME_REGEX = /\A(?!\A[\-_])(?!.*[\-_]{2,})(?!.*[\-_]\Z)[a-zA-Z0-9\-_]+\Z/
  USERNAME_REGEX_JS = USERNAME_REGEX.inspect[1..-2].gsub('\\A', '^').gsub('\\Z', '$')

  validates :username, uniqueness: { case_sensitive: false }, length: { minimum: 3, maximum: 20 },
                       format: { with: User::USERNAME_REGEX }, allow_nil: true
  validates :email, uniqueness: { case_sensitive: false }, format: { with: URI::MailTo::EMAIL_REGEXP }, allow_nil: true

  validate_aptos_address :mainnet_address

  validates :terms_accepted, acceptance: true

  has_many :authorizations, dependent: :destroy
  has_one :it2_profile, dependent: :destroy
  has_one :it2_survey, dependent: :destroy
  has_many :nfts, dependent: :destroy

  def self.from_omniauth(auth, current_user = nil)
    # find an existing user or create a user and authorizations
    # schema of auth https://github.com/omniauth/omniauth/wiki/Auth-Hash-Schema

    # returning users
    authorization = Authorization.find_by(provider: auth.provider, uid: auth.uid)
    return authorization.user if authorization

    # if user is already logged in, add new oauth to existing user
    if current_user
      current_user.add_oauth_authorization(auth).save!
      return current_user
    end

    # Totally new user
    user = create_new_user_from_oauth(auth)
    user.save!
    user
  end

  def self.create_new_user_from_oauth(auth)
    # Create a blank user: no email or username
    user = User.new({
                      password: Devise.friendly_token(32)
                    })
    user.add_oauth_authorization(auth)
    user
  end

  # Maintaining state if a user was not able to be saved
  # def self.new_with_session(params, session)
  #   super.tap do |user|
  #     if (data = session['devise.oauth.data'])
  #       user.email = data['info']['email'] if user.email.blank?
  #       user.add_oauth_authorization(data)
  #     end
  #   end
  # end

  def maybe_send_ait2_registration_complete_email
    SendRegistrationCompleteEmailJob.perform_now({ user_id: id }) if ait2_registration_complete?
  end

  def ait2_registration_complete?
    kyc_complete? && it2_profile&.validator_verified?
  end

  def kyc_complete?
    kyc_exempt? || kyc_status == 'completed'
  end

  def add_oauth_authorization(data)
    expires_at = begin
      Time.at(data['credentials']['expires_at'])
    rescue StandardError
      nil
    end
    auth = {
      provider: data['provider'],
      uid: data['uid'],
      token: data['credentials']['token'],
      expires: data['credentials']['expires'],
      secret: data['credentials']['secret'],
      refresh_token: data['credentials']['refresh_token'],
      expires_at:,
      email: data.dig('info', 'email')&.downcase,
      full_name: data.dig('info', 'name'),
      profile_url: data.dig('info', 'image')
    }
    case data['provider']
    when 'github'
      auth = auth.merge({
                          username: data.dig('info', 'nickname')&.downcase
                        })
    when 'discord'
      raw_info = data['extra']['raw_info']
      auth = auth.merge({
                          username: "#{raw_info['username'].downcase}##{raw_info['discriminator']}"
                        })
    when 'google'
      # No additional data from Google. But we can trust the email!
      if !confirmed? && !User.exists?(email: auth[:email])
        self.email = auth[:email]
        confirm
      end
    else
      raise 'Unknown Provider!'
    end
    authorizations.build(auth)
  end

  def registration_completed?
    confirmed? && username.present?
  end

  private

  # This is to allow username instead of email login in devise (for aptos admins)
  def email_required?
    false
  end

  # Use mailchimp instead of the default devise confirmation email.
  def send_confirmation_instructions
    generate_confirmation_token! unless @raw_confirmation_token

    url_options = Rails.application.config.action_mailer.default_url_options
    url = Rails.application.routes.url_helpers.user_confirmation_url(**url_options,
                                                                     confirmation_token: @raw_confirmation_token)
    SendConfirmEmailJob.perform_now({ user_id: id, template_vars: { CONFIRM_LINK: url } })
  end
end
