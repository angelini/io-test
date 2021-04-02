#!/usr/bin/env bash

set -euo pipefail

log() {
    echo "$(date +"%H:%M:%S") - $(printf '%s' "$@")" 1>&2
}

main() {
    "${HOME}/split.sh"

    log "start nginx"
    nginx
}

main "$@"
