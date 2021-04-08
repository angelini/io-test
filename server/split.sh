#!/usr/bin/env bash
# shellcheck disable=SC2155

set -euo pipefail

readonly INPUT_DIR="/mnt/data"
readonly OUTPUT_DIR="${HOME}/output"

readonly CHUNK_COUNT=4

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

    local files="$(find "${INPUT_DIR}" -type f -exec du -a {} + | sort -r | awk '{ print $2 }' | tail -n +2)"

    for file in ${files}; do
        local smallest_dir="$(du -d 1 "${OUTPUT_DIR}" | sort -n | head -n 1 | awk '{ print $2 }')"

        local relative_file="${file#$INPUT_DIR}"
        local base="$(basename "${file}")"
        local parent_dir="${smallest_dir}${relative_file%$base}"

        mkdir -p "${parent_dir}"
        cp -r "${file}" "${parent_dir}/"
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
