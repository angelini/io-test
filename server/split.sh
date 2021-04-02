#!/usr/bin/env bash
# shellcheck disable=SC2155

set -euo pipefail

readonly INPUT_DIR="/mnt/data"
readonly OUTPUT_DIR="${HOME}/output"

readonly CHUNK_COUNT=5

log() {
    echo "$(date +"%H:%M:%S") - $(printf '%s' "$@")" 1>&2
}

reset_output_dir() {
    log "reset output directory ${OUTPUT_DIR}"

    rm -rf "${OUTPUT_DIR}"
    mkdir "${OUTPUT_DIR}"

    for chunk_idx in $(seq "${CHUNK_COUNT}"); do
        mkdir -p "${OUTPUT_DIR}/${chunk_idx}"
    done
}

split_input() {
    log "split input directory ${INPUT_DIR} into ${CHUNK_COUNT} chunks"

    local directories="$(du -h -d 1 "${INPUT_DIR}" | sort -h -r | awk '{ print $2 }' | tail -n +2)"
    local chunk_idx=1

    for dir in ${directories}; do
        cp -r "${dir}" "${OUTPUT_DIR}/${chunk_idx}/"
        ((chunk_idx=chunk_idx+1))

        if [[ "${chunk_idx}" > "${CHUNK_COUNT}" ]]; then
            chunk_idx=1
        fi
    done
}

compress_chunks() {
    log "compress chunks"

    for chunk_idx in $(seq "${CHUNK_COUNT}"); do
        tar -C "${OUTPUT_DIR}/${chunk_idx}" -acf "${OUTPUT_DIR}/${chunk_idx}.tar.zst" "."
    done
}

main() {
    log "build chunks from ${INPUT_DIR}"

    reset_output_dir
    split_input
    compress_chunks
}

main "$@"
