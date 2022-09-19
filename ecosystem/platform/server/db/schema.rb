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

ActiveRecord::Schema[7.0].define(version: 2022_09_16_125743) do
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

  create_table "active_storage_attachments", force: :cascade do |t|
    t.string "name", null: false
    t.string "record_type", null: false
    t.bigint "record_id", null: false
    t.bigint "blob_id", null: false
    t.datetime "created_at", null: false
    t.index ["blob_id"], name: "index_active_storage_attachments_on_blob_id"
    t.index ["record_type", "record_id", "name", "blob_id"], name: "index_active_storage_attachments_uniqueness", unique: true
  end

  create_table "active_storage_blobs", force: :cascade do |t|
    t.string "key", null: false
    t.string "filename", null: false
    t.string "content_type"
    t.text "metadata"
    t.string "service_name", null: false
    t.bigint "byte_size", null: false
    t.string "checksum"
    t.datetime "created_at", null: false
    t.index ["key"], name: "index_active_storage_blobs_on_key", unique: true
  end

  create_table "active_storage_variant_records", force: :cascade do |t|
    t.bigint "blob_id", null: false
    t.string "variation_digest", null: false
    t.index ["blob_id", "variation_digest"], name: "index_active_storage_variant_records_uniqueness", unique: true
  end

  create_table "articles", force: :cascade do |t|
    t.string "title", null: false
    t.string "slug", null: false
    t.text "content", null: false
    t.string "status", default: "draft", null: false
    t.datetime "created_at"
    t.datetime "updated_at"
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

  create_table "categories", force: :cascade do |t|
    t.string "title", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
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
    t.string "account_address"
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
    t.string "nhc_job_id"
    t.text "nhc_output"
    t.string "account_address"
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

  create_table "it3_profiles", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "owner_key"
    t.string "consensus_key"
    t.string "account_key"
    t.string "network_key"
    t.string "validator_ip"
    t.string "validator_address"
    t.integer "validator_port"
    t.integer "validator_metrics_port"
    t.integer "validator_api_port"
    t.boolean "validator_verified", default: false, null: false
    t.string "fullnode_address"
    t.integer "fullnode_port"
    t.string "fullnode_network_key"
    t.boolean "terms_accepted", default: false, null: false
    t.boolean "selected", default: false, null: false, comment: "Whether this node is selected for participation in IT3."
    t.boolean "validator_verified_final"
    t.jsonb "metrics_data"
    t.string "nhc_job_id"
    t.text "nhc_output"
    t.string "account_address"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.string "consensus_pop"
    t.string "owner_address"
    t.integer "fullnode_metrics_port"
    t.integer "fullnode_api_port"
    t.index ["account_address"], name: "index_it3_profiles_on_account_address", unique: true
    t.index ["account_key"], name: "index_it3_profiles_on_account_key", unique: true
    t.index ["consensus_key"], name: "index_it3_profiles_on_consensus_key", unique: true
    t.index ["fullnode_network_key"], name: "index_it3_profiles_on_fullnode_network_key", unique: true
    t.index ["network_key"], name: "index_it3_profiles_on_network_key", unique: true
    t.index ["owner_key"], name: "index_it3_profiles_on_owner_key", unique: true
    t.index ["user_id"], name: "index_it3_profiles_on_user_id", unique: true
  end

  create_table "it3_surveys", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "persona", null: false
    t.string "participate_reason", null: false
    t.string "qualified_reason", null: false
    t.string "website"
    t.string "interest_reason", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["user_id"], name: "index_it3_surveys_on_user_id"
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

  create_table "network_operations", force: :cascade do |t|
    t.string "title", null: false
    t.text "content", null: false
    t.datetime "created_at"
    t.datetime "updated_at"
    t.datetime "notified_at", comment: "The time at which a notification was sent for this network operation."
  end

  create_table "nft_images", force: :cascade do |t|
    t.string "slug", null: false
    t.integer "image_number"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["slug", "image_number"], name: "index_nft_images_on_slug_and_image_number", unique: true
  end

  create_table "notification_preferences", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.integer "delivery_method", default: 0, null: false
    t.boolean "node_upgrade_notification", default: false, null: false
    t.boolean "governance_proposal_notification", default: false, null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["user_id", "delivery_method"], name: "index_notification_preferences_on_user_id_and_delivery_method", unique: true
    t.index ["user_id"], name: "index_notification_preferences_on_user_id"
  end

  create_table "notifications", force: :cascade do |t|
    t.string "recipient_type", null: false
    t.bigint "recipient_id", null: false
    t.string "type", null: false
    t.jsonb "params"
    t.datetime "read_at"
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["read_at"], name: "index_notifications_on_read_at"
    t.index ["recipient_type", "recipient_id"], name: "index_notifications_on_recipient"
  end

  create_table "project_categories", force: :cascade do |t|
    t.bigint "project_id", null: false
    t.bigint "category_id", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["category_id", "project_id"], name: "index_project_categories_on_category_id_and_project_id", unique: true
    t.index ["category_id"], name: "index_project_categories_on_category_id"
    t.index ["project_id"], name: "index_project_categories_on_project_id"
  end

  create_table "project_members", force: :cascade do |t|
    t.bigint "project_id", null: false
    t.bigint "user_id", null: false
    t.string "role", null: false
    t.boolean "public", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["project_id", "user_id"], name: "index_project_members_on_project_id_and_user_id", unique: true
    t.index ["project_id"], name: "index_project_members_on_project_id"
    t.index ["user_id"], name: "index_project_members_on_user_id"
  end

  create_table "projects", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "title", null: false
    t.string "short_description", null: false
    t.string "full_description", null: false
    t.string "website_url", null: false
    t.string "github_url"
    t.string "discord_url"
    t.string "twitter_url"
    t.string "telegram_url"
    t.string "linkedin_url"
    t.string "youtube_url"
    t.string "forum_url"
    t.boolean "public", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.boolean "verified", default: false, null: false
    t.index ["user_id"], name: "index_projects_on_user_id"
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
    t.integer "discourse_id"
    t.string "bio"
    t.index "lower((username)::text)", name: "index_users_on_lower_username", unique: true
    t.index ["confirmation_token"], name: "index_users_on_confirmation_token", unique: true
    t.index ["current_sign_in_ip"], name: "index_users_on_current_sign_in_ip"
    t.index ["email"], name: "index_users_on_email", unique: true
    t.index ["external_id"], name: "index_users_on_external_id"
    t.index ["last_sign_in_ip"], name: "index_users_on_last_sign_in_ip"
    t.index ["reset_password_token"], name: "index_users_on_reset_password_token", unique: true
  end

  create_table "wallets", force: :cascade do |t|
    t.bigint "user_id", null: false
    t.string "network", null: false, comment: "The network that the account exists on (e.g. 'ait3')."
    t.string "wallet_name", null: false, comment: "The name of the wallet (e.g. 'petra')."
    t.string "public_key", null: false, comment: "The public key of the account."
    t.string "address", null: false, comment: "The account address."
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.index ["public_key", "network", "wallet_name"], name: "index_wallets_on_public_key_and_network_and_wallet_name", unique: true
    t.index ["user_id"], name: "index_wallets_on_user_id"
    t.check_constraint "public_key::text ~ '^0x[0-9a-f]{64}$'::text"
  end

  add_foreign_key "active_storage_attachments", "active_storage_blobs", column: "blob_id"
  add_foreign_key "active_storage_variant_records", "active_storage_blobs", column: "blob_id"
  add_foreign_key "it1_profiles", "users"
  add_foreign_key "it2_profiles", "users"
  add_foreign_key "it2_surveys", "users"
  add_foreign_key "it3_profiles", "users"
  add_foreign_key "it3_surveys", "users"
  add_foreign_key "notification_preferences", "users"
  add_foreign_key "project_categories", "categories"
  add_foreign_key "project_categories", "projects"
  add_foreign_key "project_members", "projects"
  add_foreign_key "project_members", "users"
  add_foreign_key "projects", "users"
  add_foreign_key "wallets", "users"
end
