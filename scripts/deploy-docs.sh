#!/bin/bash -e
source ${HOME}/google-cloud-sdk/path.bash.inc && \
cargo doc --no-deps && \
mkdir -p docs/site/rustdoc && \
mv target/doc/* docs/site/rustdoc && \
(gsutil -m rm -rf gs://http-gateway.docs.netology-group.services/* || true) && \
gsutil -m mv -ra public-read docs/site/* \
    gs://http-gateway.docs.netology-group.services && \
rm -rf docs/site/rustdoc
