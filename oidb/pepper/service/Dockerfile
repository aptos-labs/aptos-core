FROM rust:1.74.1 as builder
ARG GIT_COMMIT
WORKDIR /app
COPY . .
RUN cargo build --release -p aptos-oidb-pepper-service

FROM rust:1.74.1
COPY --from=builder /app/target/release/aptos-oidb-pepper-service /usr/local/bin
ENV GIT_COMMIT=$GIT_COMMIT
EXPOSE 8000
ENTRYPOINT ["aptos-oidb-pepper-service"]
