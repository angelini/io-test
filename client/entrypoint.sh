#!/usr/bin/env bash

set -euo pipefail

readonly OUTPUT_DIR="/mnt/data"

log() {
    echo "$(date +"%H:%M:%S") - $(printf '%s' "$@")" 1>&2
}

run_rust() {
    log "running fs-rebuild"
    rm -rf "${OUTPUT_DIR:?}/*"
    time "${HOME}/fs-rebuild"
}

run_shell() {
    log "running rebuild.sh"
    rm -rf "${OUTPUT_DIR:?}/*"
    time "${HOME}/rebuild.sh"
}

main() {
    mkdir -p "${OUTPUT_DIR}"

    for _ in $(seq 5); do
        run_rust
        run_shell
    done
}

main "$@"
