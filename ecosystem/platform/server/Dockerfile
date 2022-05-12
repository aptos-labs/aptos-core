# syntax=docker/dockerfile:1.4

FROM golang:1.18.2-buster@sha256:72506be8130fd4c98a032a497db82234aaace7eb48a90aee161e821c767111f1 as aws-env

# build aws-env which is used to inject load AWS Secrets as environment variables at init time.
RUN git clone --depth=1 --branch v1.3.0 https://github.com/aptos-labs/aws-env src \
    && cd src/cmd/aws-env \
    && CGO_ENABLED=0 GOARCH=amd64 GOOS=linux go build -ldflags '-w -s' -o /aws-env && chmod +x /aws-env

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

COPY --link Gemfile Gemfile.lock ./

ENV RAILS_ENV production
ENV RAILS_SERVE_STATIC_FILES true
ENV RAILS_LOG_TO_STDOUT true

RUN bundle config --global frozen 1
RUN bundle config set --local without 'development test'
RUN bundle install

COPY --link . /usr/src/app

# this is a fake secret key just to get it to compile the assets
ENV SECRET_KEY_BASE=18ff7d41b9da02d394434e2eb140611516707334b7d7a7e15cf8b567061f30e9400a82e61aa772b9e6ccb72853932769d03bbbb9d78f62333c8f7adb95cc727d

# Yes, this should run twice
# https://github.com/rails/tailwindcss-rails/issues/158
RUN SKIP_DB_CHECK=1 bundle exec rake assets:precompile && SKIP_DB_CHECK=1 bundle exec rake assets:precompile

COPY --link --from=aws-env /aws-env /aws-env

EXPOSE 3000

CMD ["/aws-env", "exec", "--", "bundle", "exec", "rails", "s", "-b", "0.0.0.0"]
