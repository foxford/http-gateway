#!/bin/bash

if [ ! -d "$HOME/google-cloud-sdk/bin" ]; then
    rm -rf $HOME/google-cloud-sdk;
    curl https://sdk.cloud.google.com | bash;
fi && \
$HOME/google-cloud-sdk/install.sh -q && \
source $HOME/google-cloud-sdk/path.bash.inc && \
openssl aes-256-cbc \
    -K $encrypted_379ae646eb9b_key \
    -iv $encrypted_379ae646eb9b_iv \
    -in .travis-key.json.enc \
    -out .travis-key.json -d && \
gcloud auth activate-service-account --key-file .travis-key.json && \
rm -f .travis-key.json && \
cargo doc --no-deps && \
mkdir -p docs/site/rustdoc && \
mv target/doc/* docs/site/rustdoc && \
(gsutil -m rm -rf gs://http-gateway.docs.netology-group.services/* || true) && \
gsutil -m mv -ra public-read docs/site/* \
    gs://http-gateway.docs.netology-group.services && \
rm -rf docs/site/rustdoc