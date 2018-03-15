FROM rust:latest

WORKDIR /usr/src/http-gateway
COPY . .

RUN cargo install

CMD ["http-gateway"]