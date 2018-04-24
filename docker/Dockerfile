FROM ubuntu:16.04 as builder

WORKDIR /usr/src/http-gateway

RUN apt-get update -y && apt-get install curl build-essential libssl-dev clang git -y
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
RUN git clone https://github.com/eclipse/paho.mqtt.c.git && \
    cd paho.mqtt.c && \
    git checkout tags/v1.2.0 && \
    make && make install
ENV PATH $PATH:/root/.cargo/bin
COPY . .
RUN cargo build --release

FROM ubuntu:16.04

RUN apt-get update -y && apt-get install build-essential libssl-dev -y

COPY --from=builder /usr/src/http-gateway/target/release/http-gateway /app/
COPY --from=builder /usr/src/http-gateway/paho.mqtt.c /paho/
RUN cd /paho && make install && cd .. && rm -rf /paho

CMD ["/app/http-gateway"]
