json.extract! project, :id, :title, :short_description, :full_description, :website_url, :github_url, :discord_url, :twitter_url, :telegram_url, :linkedin_url, :thumbnail_url, :youtube_url, :forum_url, :public, :created_at, :updated_at
json.url project_url(project, format: :json)
