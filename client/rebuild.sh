#!/usr/bin/env bash

set -euo pipefail

readonly SERVER="http://${SERVER_SERVICE_HOST}:${SERVER_SERVICE_PORT}"
readonly OUTPUT_DIR="/mnt/data/foo"

readonly CHUNK_COUNT=8

log() {
    echo "$(date +"%H:%M:%S") - $(printf '%s' "$@")" 1>&2
}

fetch_chunk() {
    local idx="${1}"
    curl -fsSL "${SERVER}/${idx}.tar.zst" | tar --zstd -x -C "${OUTPUT_DIR}"
}

main() {
    log "rebuild directory into ${OUTPUT_DIR}"

    mkdir -p "${OUTPUT_DIR}"

    local pids=()

    for chunk_idx in $(seq "${CHUNK_COUNT}"); do
        fetch_chunk "${chunk_idx}" &
        pids["${chunk_idx}"]="${!}"
    done

    for pid in ${pids[*]}; do
        wait "${pid}"
    done
}

main "$@"
