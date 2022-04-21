# frozen_string_literal: true

# TODO: CSRF tokens!

Rails.application.config.middleware.use OmniAuth::Builder do
  provider :developer unless Rails.env.production?
  provider :discord, ENV.fetch('DISCORD_CLIENT_ID', nil), ENV.fetch('DISCORD_CLIENT_SECRET', nil)
  # https://github.com/omniauth/omniauth-github
  provider :github, ENV.fetch('GITHUB_KEY', nil), ENV.fetch('GITHUB_SECRET', nil), scope: 'read:user,user:email'
end
