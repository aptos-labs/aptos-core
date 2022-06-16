# syntax=docker/dockerfile:1.4

FROM phusion/passenger-full:2.3.0

ADD nginx.conf /etc/nginx/sites-enabled/webapp.conf

RUN npm install -g yarn && \
  rm -f /etc/my_init.d/10_syslog-ng.init && \
  rm -f /etc/service/nginx/down && \
  rm -f /etc/nginx/sites-enabled/default && \
  mkdir /home/app/webapp && \
  chown app: /home/app/webapp

USER app
ENV HOME /home/app
ENV RAILS_ENV production
ENV RAILS_LOG_TO_STDOUT true
ENV NODE_ENV production
WORKDIR /home/app/webapp

COPY --chown=app:app Gemfile Gemfile.lock .

RUN bundle config --global frozen 1 && \
  bundle config set --local without 'development test' && \
  bundle install

COPY --chown=app:app package.json yarn.lock .

RUN yarn install

COPY --chown=app:app . /home/app/webapp

RUN yarn build && yarn build:css && \
  SECRET_KEY_BASE=$(bin/rake secret) SKIP_DB_CHECK=1 bundle exec rake assets:precompile

EXPOSE 3000

USER root
CMD ["/sbin/my_init"]
