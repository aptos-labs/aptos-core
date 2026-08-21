FROM golang:1.22-bookworm AS builder
WORKDIR /src
COPY go.mod ./
COPY main.go ./
RUN go mod tidy
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags="-s -w" -o /attester

FROM scratch
ARG USER_DATA_HEX
ENV USER_DATA_HEX=${USER_DATA_HEX}
COPY --from=builder /attester /attester
ENTRYPOINT ["/attester"]
