FROM rust:1.75.0 as builder
ARG GIT_COMMIT
WORKDIR /app
COPY . .
RUN ./scripts/dev_setup.sh -b
RUN cargo build --release -p aptos-keyless-pepper-service

FROM rust:1.75.0
COPY --from=builder /app/target/release/aptos-keyless-pepper-service /usr/local/bin
ENV GIT_COMMIT=$GIT_COMMIT
EXPOSE 8000
ENTRYPOINT ["aptos-keyless-pepper-service"]
