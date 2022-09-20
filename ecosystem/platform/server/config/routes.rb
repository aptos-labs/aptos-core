# frozen_string_literal: true

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

Rails.application.routes.draw do
  # Redirect www to non-www
  constraints(host: /www\.aptoslabs\.com/) do
    match '/(*path)' => redirect { |params, _req| "https://aptoslabs.com/#{params[:path]}" }, via: %i[get post]
  end

  # Redirect community.aptoslabs.com to aptoslabs.com
  constraints host: /community\.aptoslabs\.com/ do
    match '/*path' => redirect { |params, _req| "https://aptoslabs.com/#{params[:path]}" }, via: %i[get post]
    match '/' => redirect { |_params, _req| 'https://aptoslabs.com/community' }, via: %i[get post]
  end

  devise_for :users, {
    controllers: {
      omniauth_callbacks: 'users/omniauth_callbacks',
      sessions: 'users/sessions',
      confirmations: 'users/confirmations'
    }
  }

  # Administration
  ActiveAdmin.routes(self)
  constraints(lambda { |request|
    user = request.env['warden'].user
    user.respond_to?(:is_root?) && user.is_root?
  }) do
    # Feature flags
    mount Flipper::UI.app(Flipper) => '/flipper'
  end

  # Define your application routes per the DSL in https://guides.rubyonrails.org/routing.html

  namespace :user do
    root to: redirect('/community') # creates user_root_path, where users go after confirming email
  end

  # CMS
  resources :articles, param: :slug, only: %i[index show]
  resources :network_operations, only: %i[index show]

  # Settings
  get 'settings', to: redirect('/settings/profile')
  get 'settings/profile'
  patch 'settings/profile', to: 'settings#profile_update'
  get 'settings/notifications'
  patch 'settings/notifications', to: 'settings#notifications_update'
  get 'settings/connections'
  delete 'settings/connections', to: 'settings#connections_delete'
  delete 'settings/delete_account', to: 'settings#delete_account'

  # Discourse SSO
  get 'discourse/sso', to: 'discourse#sso'

  # KYC routes
  get 'onboarding/kyc_redirect', to: 'onboarding#kyc_redirect'
  get 'onboarding/kyc_callback', to: 'onboarding#kyc_callback'

  # Onboarding
  get 'onboarding/email'
  get 'onboarding/email_success'
  post 'onboarding/email', to: 'onboarding#email_update'

  # Health check
  get 'health', to: 'health#health'

  # IT3
  resource :it3, only: %i[show update]
  resources :it3_profiles, except: %i[show create new index destroy]
  resources :it3_surveys, except: %i[index destroy]

  # NFTs
  resources :nft_offers, param: :slug, only: %i[show update]
  get 'nft_images/:nft_offer_slug/:image_num', to: 'nft_images#show', constraints: { image_num: /\d+/ }
  get 'n/:offer_id/:txn_version', as: 'nft_offer_short', to: 'nft_offers#short', constraints: { txn_version: /\d+/ }

  # Leaderboards
  get 'leaderboard/it1', to: redirect('/it1')
  get 'leaderboard/it2', to: redirect('/it2')
  get 'leaderboard/it3'

  # IT1
  get 'it1', to: 'leaderboard#it1'
  get 'it2', to: 'leaderboard#it2'

  # Projects
  get 'projects', to: redirect('/ecosystem')
  get 'ecosystem', to: 'projects#index'
  resources :projects, except: %i[index edit update]

  # User profiles
  resources :users, only: %i[show] do
    get 'projects'
    get 'activity'
    get 'rewards'
  end

  # Wallets
  resources :wallets, only: %i[show create]

  # Discord invite link
  get 'discord', to: redirect('https://discord.com/invite/aptoslabs')

  # Static pages
  get 'community', to: 'static_page#community'
  get 'incentivized-testnet', to: 'static_page#incentivized_testnet'
  get 'terms', to: 'static_page#terms'
  get 'terms-testnet', to: 'static_page#terms_testnet'
  get 'privacy', to: 'static_page#privacy'
  get 'developers', to: 'static_page#developers'
  get 'currents', to: 'static_page#currents'
  get 'careers', to: 'static_page#careers'
  root 'static_page#root'
end
