#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Error: File path is required as first argument"
    echo "Usage: $0 <file_path> [cli|sdl] [timeout_ms]"
    exit 1
fi

FILE_PATH="$1"
MODE="${2:-sdl}"
TIMEOUT_MS="${3:-500}"

if [ ! -f "$FILE_PATH" ]; then
    echo "Error: File '$FILE_PATH' does not exist"
    exit 1
fi

if [[ "$MODE" != "cli" && "$MODE" != "sdl" ]]; then
    echo "Error: Mode must be 'cli' or 'sdl'"
    exit 1
fi

if [ "$MODE" = "cli" ]; then
    echo "Running CLI mode with file: $FILE_PATH, timeout: ${TIMEOUT_MS}ms"
    cargo run --bin cli -- "$FILE_PATH" --debug --timeout-ms="$TIMEOUT_MS"
else
    echo "Running SDL mode with file: $FILE_PATH, timeout: ${TIMEOUT_MS}ms"
    cargo run --features sdl --bin sdl -- "$FILE_PATH" --debug --timeout-ms="$TIMEOUT_MS" --font=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
fi
