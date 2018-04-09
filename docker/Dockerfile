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
RUN git submodule update --init
RUN cargo build --release

FROM ubuntu:16.04

COPY --from=builder /usr/src/http-gateway/target/release/http-gateway /app/
COPY --from=builder /usr/src/http-gateway/paho.mqtt.c /paho/

RUN apt-get update -y && apt-get install -y build-essential libssl-dev
RUN cd /paho && make install && cd .. && rm -rf /paho

CMD ["/app/http-gateway"]
