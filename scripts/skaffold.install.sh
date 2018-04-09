#!/bin/bash -e
SKAFFOLD_CLI_PATH=${SKAFFOLD_CLI_PATH:-"/tmp"}
SKAFFOLD_VERSION=${SKAFFOLD_VERSION:-"0.3.0"}

SKAFFOLD_PLATFORM=$(if [[ $(uname) != "Darwin" ]]; then echo "linux"; else echo "darwin"; fi)

curl -Lo skaffold \
    "https://storage.googleapis.com/skaffold/releases/v${SKAFFOLD_VERSION}/skaffold-${SKAFFOLD_PLATFORM}-amd64"
chmod +x skaffold
mv skaffold ${SKAFFOLD_CLI_PATH}
