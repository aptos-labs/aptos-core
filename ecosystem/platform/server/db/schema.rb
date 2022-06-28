# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[7.0].define(version: 2022_06_27_190226) do
  # These are extensions that must be enabled in order to support this database
  enable_extension "pgcrypto"
  enable_extension "plpgsql"

  create_table "active_admin_comments", force: :cascade do |t|
    t.string "namespace"
    t.text "body"
    t.string "resource_type"
    t.bigint "resource_id"
    t.string "author_type"
    t.bigint "author_id"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["author_type", "author_id"], name: "index_active_admin_comments_on_author"
    t.index ["namespace"], name: "index_active_admin_comments_on_namespace"
    t.index ["resource_type", "resource_id"], name: "index_active_admin_comments_on_resource"
  end

  create_table "authorizations", force: :cascade do |t|
    t.integer "user_id"
    t.string "provider"
    t.string "uid"
    t.string "email"
    t.string "username"
    t.string "full_name"
    t.text "profile_url"
    t.string "token"
    t.string "secret"
    t.string "refresh_token"
    t.boolean "expires"
    t.datetime "expires_at", precision: nil
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["provider", "uid"], name: "index_authorizations_on_provider_and_uid"
    t.index ["provider"], name: "index_authorizations_on_provider"
    t.index ["uid"], name: "index_authorizations_on_uid"
    t.index ["user_id"], name: "index_authorizations_on_user_id"
  end

  create_table "delayed_jobs", force: :cascade do |t|
    t.integer "priority", default: 0, null: false
    t.integer "attempts", default: 0, null: false
    t.text "handler", null: false
    t.text "last_error"
    t.datetime "run_at"
    t.datetime "locked_at"
    t.datetime "failed_at"
    t.string "locked_by"
    t.string "queue"
    t.datetime "created_at"
    t.datetime "updated_at"
    t.index ["priority", "run_at"], name: "delayed_jobs_priority"
  end

  create_table "flipper_features", force: :cascade do |t|
    t.string "key", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["key"], name: "index_flipper_features_on_key", unique: true
  end

  create_table "flipper_gates", force: :cascade do |t|
    t.string "feature_key", null: false
    t.string "key", null: false
    t.string "value"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["feature_key", "key", "value"], name: "index_flipper_gates_on_feature_key_and_key_and_value", unique: true
  end

  create_table "it1_profiles", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "consensus_key"
    t.string "account_key"
    t.string "network_key"
    t.string "validator_ip"
    t.string "validator_address"
    t.integer "validator_port"
    t.integer "validator_metrics_port"
    t.integer "validator_api_port"
    t.boolean "validator_verified", default: false
    t.string "fullnode_address"
    t.integer "fullnode_port"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.boolean "terms_accepted", default: false
    t.string "fullnode_network_key"
    t.boolean "selected", default: false, null: false, comment: "Whether this node is selected for participation in IT1."
    t.boolean "validator_verified_final"
    t.jsonb "metrics_data"
    t.index ["user_id"], name: "index_it1_profiles_on_user_id"
  end

  create_table "it2_profiles", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "consensus_key", null: false
    t.string "account_key", null: false
    t.string "network_key", null: false
    t.string "validator_ip"
    t.string "validator_address", null: false
    t.integer "validator_port", null: false
    t.integer "validator_metrics_port", null: false
    t.integer "validator_api_port", null: false
    t.boolean "validator_verified", default: false, null: false
    t.string "fullnode_address"
    t.integer "fullnode_port"
    t.string "fullnode_network_key"
    t.boolean "terms_accepted", default: false, null: false
    t.boolean "selected", default: false, null: false, comment: "Whether this node is selected for participation in IT2."
    t.boolean "validator_verified_final"
    t.jsonb "metrics_data"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["account_key"], name: "index_it2_profiles_on_account_key", unique: true
    t.index ["consensus_key"], name: "index_it2_profiles_on_consensus_key", unique: true
    t.index ["fullnode_network_key"], name: "index_it2_profiles_on_fullnode_network_key", unique: true
    t.index ["network_key"], name: "index_it2_profiles_on_network_key", unique: true
    t.index ["user_id"], name: "index_it2_profiles_on_user_id", unique: true
  end

  create_table "it2_surveys", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "persona", null: false
    t.string "participate_reason", null: false
    t.string "qualified_reason", null: false
    t.string "website"
    t.string "interest_reason", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["user_id"], name: "index_it2_surveys_on_user_id"
  end

  create_table "locations", force: :cascade do |t|
    t.string "item_type", null: false
    t.bigint "item_id", null: false
    t.integer "accuracy_radius"
    t.integer "average_income"
    t.float "latitude"
    t.float "longitude"
    t.integer "metro_code"
    t.integer "population_density"
    t.string "time_zone"
    t.boolean "anonymous"
    t.boolean "anonymous_vpn"
    t.integer "autonomous_system_number"
    t.string "autonomous_system_organization"
    t.string "connection_type"
    t.string "domain"
    t.boolean "hosting_provider"
    t.string "ip_address"
    t.string "isp"
    t.boolean "legitimate_proxy"
    t.string "mobile_country_code"
    t.string "mobile_network_code"
    t.string "network"
    t.string "organization"
    t.boolean "public_proxy"
    t.boolean "residential_proxy"
    t.float "static_ip_score"
    t.boolean "tor_exit_node"
    t.integer "user_count"
    t.string "user_type"
    t.string "continent_code"
    t.string "continent_geoname_id"
    t.string "continent_name"
    t.integer "country_confidence"
    t.string "country_geoname_id"
    t.string "country_iso_code"
    t.string "country_name"
    t.integer "subdivision_confidence"
    t.string "subdivision_geoname_id"
    t.string "subdivision_iso_code"
    t.string "subdivision_name"
    t.integer "city_confidence"
    t.string "city_geoname_id"
    t.string "city_name"
    t.integer "postal_confidence"
    t.string "postal_code"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["item_type", "item_id"], name: "index_locations_on_item"
  end

  create_table "nft_offers", force: :cascade do |t|
    t.string "name", null: false
    t.datetime "valid_from"
    t.datetime "valid_until"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["name"], name: "index_nft_offers_on_name", unique: true
  end

  create_table "nfts", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.bigint "nft_offer_id", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.string "explorer_url"
    t.index ["nft_offer_id"], name: "index_nfts_on_nft_offer_id"
    t.index ["user_id"], name: "index_nfts_on_user_id"
  end

  create_table "users", force: :cascade do |t|
    t.string "username"
    t.string "email"
    t.string "encrypted_password", default: "", null: false
    t.string "reset_password_token"
    t.datetime "reset_password_sent_at"
    t.datetime "remember_created_at"
    t.integer "sign_in_count", default: 0, null: false
    t.datetime "current_sign_in_at"
    t.datetime "last_sign_in_at"
    t.string "current_sign_in_ip"
    t.string "last_sign_in_ip"
    t.string "confirmation_token"
    t.datetime "confirmed_at"
    t.datetime "confirmation_sent_at"
    t.string "unconfirmed_email"
    t.boolean "is_root", default: false, null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.boolean "is_developer", default: false, null: false
    t.boolean "is_node_operator", default: false, null: false
    t.string "mainnet_address"
    t.string "kyc_status", default: "not_started", null: false
    t.uuid "external_id", default: -> { "gen_random_uuid()" }, null: false
    t.boolean "kyc_exempt", default: false
    t.string "completed_persona_inquiry_id"
    t.index ["confirmation_token"], name: "index_users_on_confirmation_token", unique: true
    t.index ["current_sign_in_ip"], name: "index_users_on_current_sign_in_ip"
    t.index ["email"], name: "index_users_on_email", unique: true
    t.index ["external_id"], name: "index_users_on_external_id"
    t.index ["last_sign_in_ip"], name: "index_users_on_last_sign_in_ip"
    t.index ["reset_password_token"], name: "index_users_on_reset_password_token", unique: true
    t.index ["username"], name: "index_users_on_username", unique: true
  end

  add_foreign_key "it1_profiles", "users"
  add_foreign_key "it2_profiles", "users"
  add_foreign_key "it2_surveys", "users"
  add_foreign_key "nfts", "nft_offers"
  add_foreign_key "nfts", "users"
end
