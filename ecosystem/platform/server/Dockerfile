FROM ruby:3.1.2-slim

RUN apt-get update -qq && apt-get install -yq --no-install-recommends \
    build-essential \
    gnupg2 \
    less \
    git \
    libpq-dev \
    postgresql-client \
  && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN gem update --system && gem install bundler

WORKDIR /usr/src/app

COPY Gemfile Gemfile.lock ./

ENV RAILS_ENV production
ENV RAILS_SERVE_STATIC_FILES true
ENV RAILS_LOG_TO_STDOUT true

RUN bundle config --global frozen 1
RUN bundle config set --local without 'development test'
RUN bundle install

COPY . /usr/src/app

# this is a fake secret key just to get it to compile the assets
ENV SECRET_KEY_BASE=18ff7d41b9da02d394434e2eb140611516707334b7d7a7e15cf8b567061f30e9400a82e61aa772b9e6ccb72853932769d03bbbb9d78f62333c8f7adb95cc727d

# Yes, this should run twice
# https://github.com/rails/tailwindcss-rails/issues/158
RUN bundle exec rake assets:precompile && bundle exec rake assets:precompile

EXPOSE 3000
CMD ["bundle", "exec", "rails", "s", "-b", "0.0.0.0"]
