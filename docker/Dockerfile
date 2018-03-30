FROM ubuntu:16.04

WORKDIR /usr/src/http-gateway

RUN apt-get update -y && apt-get install curl build-essential libssl-dev clang git -y
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
RUN git clone https://github.com/eclipse/paho.mqtt.c.git && \
    cd paho.mqtt.c && \
    git checkout tags/v1.2.0 && \
    make && make install
ENV PATH $PATH:/root/.cargo/bin
COPY . .
RUN cargo install

CMD ["mqtt-bridge"]
