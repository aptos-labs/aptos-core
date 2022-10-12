# HTTP Tester 
creates an axum server behind an haproxy to test http configs for haproxy

## Server
1. cargo build --release
2. cd docker
3. docker compose up

## Test
1. curl <ip>:8180
