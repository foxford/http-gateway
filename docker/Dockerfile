FROM rust:latest AS builder

WORKDIR /usr/src/http-gateway

COPY . .
RUN cargo build --release

FROM debian:stretch

COPY --from=builder /usr/src/http-gateway/target/release/http-gateway /app/

WORKDIR "/app"
CMD ["/app/http-gateway"]
