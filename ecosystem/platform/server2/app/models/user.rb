# frozen_string_literal: true

class User < ApplicationRecord
  include RailsStateMachine::Model

  # Include default devise modules. Others available are:
  # :confirmable, :lockable, :timeoutable, :recoverable,
  devise :database_authenticatable,
         :rememberable, :trackable, :validatable,
         :omniauthable, omniauth_providers: %i[discord github],
                        authentication_keys: [:username]

  validates :username, presence: true, uniqueness: { case_sensitive: false }

  validate_hex :mainnet_address

  has_many :authorizations, dependent: :destroy

  # https://github.com/makandra/rails_state_machine
  # TODO: this state machine
  state_machine :kyc_status do
    state :not_started, initial: true
  end

  def self.from_omniauth(auth, current_user = nil)
    # find an existing user or create a user and authorizations
    # schema of auth https://github.com/omniauth/omniauth/wiki/Auth-Hash-Schema

    # returning users
    authorization = Authorization.find_by(provider: auth.provider, uid: auth.uid)
    return authorization.user if authorization

    email = auth['info']['email']

    # if user is already logged in, add new oauth to existing user
    if current_user
      current_user.add_oauth_authorization(auth).save!
      return current_user
    end

    # Totally new user
    create_new_user_from_oauth(auth, email)
  end

  def self.create_new_user_from_oauth(auth, email)
    user = User.new({
                      # TODO: If this username is taken, should add random suffix to not explode
                      username: email.split('@').first.gsub('.', ''),
                      password: Devise.friendly_token[0, 20]
                    })
    # user.skip_confirmation! if %w[google].include?(auth.provider)
    user.add_oauth_authorization(auth)
    user.save
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

  def providers
    authorizations.map(&:provider)
  end

  def add_oauth_authorization(data)
    expires_at = begin
      Time.at(data['credentials']['expires_at'])
    rescue StandardError
      nil
    end
    authorizations.build({
                           provider: data['provider'],
                           uid: data['uid'],
                           token: data['credentials']['token'],
                           secret: data['credentials']['secret'],
                           refresh_token: data['credentials']['refresh_token'],
                           expires: data['credentials']['expires'],
                           expires_at:,
                           # Human readable label if a user connects multiple accounts
                           email: data['info']['email']
                         })
  end

  private

  # This is to allow username instead of email login in devise (for aptos admins)
  def email_required?
    false
  end

  # This is to allow username instead of email login in devise (for aptos admins)
  def will_save_change_to_email?
    false
  end
end
