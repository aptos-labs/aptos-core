# https://stackoverflow.com/a/14061796/2237879
#
# This hack allows you to run make commands with any set of arguments.
#
# For example, these lines are the same:
#   > make g devise:install
#   > bundle exec rails generate devise:install
# And these:
#   > make add-migration add_deleted_at_to_users deleted_at:datetime
#   > bundle exec rails g migration add_deleted_at_to_users deleted_at:datetime
# And these:
#   > make add-model Order user:references record:references{polymorphic}
#   > bundle exec rails g model Order user:references record:references{polymorphic}
#
RUN_ARGS := $(wordlist 2, $(words $(MAKECMDGOALS)), $(MAKECMDGOALS))

# Figure out which shell we're in
ifneq ("$(wildcard ~/.zshrc)","")
    SHELL_NAME=zsh
    SHELL_PATH=~/.zshrc
else
    SHELL_NAME=bash
    SHELL_PATH=~/.bash_profile
endif

# Check to see if this is hitting a prod DB
ifeq ($(findstring prod, $(COMMUNITY_DB_NAME)), prod)
	IS_PROD=1
else
	IS_PROD=0
endif

setup-osx:
	@#brew install postgresql rbenv ruby-build

	@# Put rbenv init in bash profile
	@grep -qsF 'rbenv init' $(SHELL_PATH); \
	if [ $$? -ne 0 ]; \
	then \
		echo "Adding rbenv init to ${SHELL_PATH} ..."; \
		echo 'eval "$$(rbenv init - $(SHELL_NAME))"' >> $(SHELL_PATH); \
	else \
		echo "${SHELL_PATH} already contains rbenv init, skipping..."; \
	fi

	@# Install ruby version
	@rbenv versions | grep 3.1.2
	@if [ $$? -ne 0 ]; \
	then \
		echo "Installing ruby 3.1.2 via rbenv..."; \
		rbenv rehash; \
		rbenv install 3.1.2; \
		rbenv rehash; \
		rbenv global 3.1.2; \
		rbenv local 3.1.2; \
		rbenv rehash; \
	else \
	  		rbenv local 3.1.2; \
		echo "Ruby 3.1.2 is already installed, skipping..."; \
	fi


	@# Install ruby gems
	@echo "Running bundler to install gems..."
	@bundle

add-migration:
	bundle exec rails g migration $(RUN_ARGS)

add-model:
	bundle exec rails g model $(RUN_ARGS)



check-not-prod:
	@if [ "$(IS_PROD)" == 1 ]; \
	  then \
	    echo "YOU ARE ATTEMPTING TO RUN AGAINST DB '$(COMMUNITY_DB_NAME)' - THIS COMMAND CAN NOT BE RUN IN PRODUCTION!"; \
		exit 2; \
	fi

db-reset: check-not-prod
	bundle exec rake db:nuke
	bundle exec rake db:migrate
	bundle exec rails db:seed

db-create:
	bundle exec rake db:create

db-migrate:
	bundle exec rake db:migrate

db-rollback:
	bundle exec rake db:rollback



lint-ruby:
	bundle exec rubocop -a

lint-security:
	bundle exec brakeman

lint: lint-ruby lint-security


run-console:
	bundle exec rails console

run-generate:
	bundle exec rails generate $(RUN_ARGS)

run-rails:
	bundle exec puma -t 1:1 -b tcp://0.0.0.0:3001

run-server:
	bundle exec rails s

make mail:
	# local SMTP testing
	docker run -p 1080:1080 -p 1025:1025 stpaquet/alpinemailcatcher:latest


c: run-console

g: run-generate

s: run-server
