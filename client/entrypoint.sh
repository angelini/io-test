#!/usr/bin/env bash

set -euo pipefail

readonly OUTPUT_DIR="/mnt/data"

export HOME="/home/main"

log() {
    echo "$(date +"%H:%M:%S") - $(printf '%s' "$@")" 1>&2
}

run_rust() {
    log "running fs-rebuild"
    rm -rf "${OUTPUT_DIR:?}/*"
    time "${HOME}/fs-rebuild" --chunks 8 \
        rebuild --host "http://${SERVER_SERVICE_HOST}:${SERVER_SERVICE_PORT}" --output "${OUTPUT_DIR}"
}

run_shell() {
    log "running rebuild.sh"
    rm -rf "${OUTPUT_DIR:?}/*"
    time "${HOME}/rebuild.sh"
}

main() {
    mkdir -p "${OUTPUT_DIR}"

    for _ in $(seq 20); do
        run_shell
        du -sh "${OUTPUT_DIR}"
        run_rust
        du -sh "${OUTPUT_DIR}"
    done
}

main "$@"
