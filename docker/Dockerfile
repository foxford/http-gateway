## -----------------------------------------------------------------------------
## Build
## -----------------------------------------------------------------------------
FROM clux/muslrust:stable as build-stage

WORKDIR "/build"
COPY . .
RUN cargo build --release

## -----------------------------------------------------------------------------
## Package
## -----------------------------------------------------------------------------
FROM alpine

RUN apk update && apk upgrade && apk add ca-certificates openssl
RUN sed -i 's/mozilla\/DST_Root_CA_X3.crt/!mozilla\/DST_Root_CA_X3.crt/g' /etc/ca-certificates.conf && update-ca-certificates

COPY --from=build-stage "/build/target/x86_64-unknown-linux-musl/release/http-gateway" "/app/http-gateway"

WORKDIR "/app"
ENTRYPOINT ["/app/http-gateway"]
