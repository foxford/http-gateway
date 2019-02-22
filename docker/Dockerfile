## -----------------------------------------------------------------------------
## Build
## -----------------------------------------------------------------------------
FROM clux/muslrust as build-stage

WORKDIR "/build"
COPY . .
RUN cargo build --release

## -----------------------------------------------------------------------------
## Package
## -----------------------------------------------------------------------------
FROM alpine

COPY --from=build-stage "/build/target/x86_64-unknown-linux-musl/release/http-gateway" "/app/http-gateway"

WORKDIR "/app"
ENTRYPOINT ["/app/http-gateway"]