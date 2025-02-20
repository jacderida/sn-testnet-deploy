#!/usr/bin/env bash

CONTACT_PEER="${1:-}"
NETWORK_CONTACTS_URL="${2:-}"
NETWORK_ID="${3:-}"

CONTACT_PEER_ARG=""
if [[ -n "$CONTACT_PEER" ]]; then
  CONTACT_PEER_ARG="--peer $CONTACT_PEER"
fi
NETWORK_CONTACTS_URL_ARG=""
if [[ -n "$NETWORK_CONTACTS_URL" ]]; then
  NETWORK_CONTACTS_URL_ARG="--network-contacts-url $NETWORK_CONTACTS_URL"
fi
NETWORK_ID_ARG=""
if [[ -n "$NETWORK_ID" ]]; then
  NETWORK_ID_ARG="--network-id $NETWORK_ID"
fi

if ! command -v ant &> /dev/null; then
  echo "Error: 'ant' not found in PATH."
  exit 1
fi

LOG_FILE="./uploaded_files.log"
DOWNLOAD_DIR="./downloaded_files"
SLEEP_INTERVAL=5

mkdir -p "$DOWNLOAD_DIR"

download_file() {
  local file_ref=$1
  # Multiple downloaders can be running on the same machine, so one or more
  # could select the same file address at the same time. We therefore use a GUID
  # to ensure a unique output file.
  local output_path="$(uuidgen).dat"
  (
    cd "$DOWNLOAD_DIR"
    echo "Downloading file: $file_ref"
    ant $CONTACT_PEER_ARG $NETWORK_CONTACTS_URL_ARG $NETWORK_ID_ARG files download "$output_path" "$file_ref"
    if [[ $? -eq 0 ]]; then
      echo "Downloaded $file_ref to $output_path"
      # Keeping these files could cause the disk to become full quite quickly, so just delete them.
      rm "$output_path"
    else
      echo "Failed to download $file_ref"
    fi
  )
}

while true; do
  if [[ -f "$LOG_FILE" && -s "$LOG_FILE" ]]; then
    file_ref=$(shuf -n 1 "$LOG_FILE")
    if [[ -n "$file_ref" ]]; then
      download_file "$file_ref"
      sleep 5
    else
      echo "Selected line is empty. Retrying..."
    fi
  else
    echo "Log file '$LOG_FILE' does not exist or is empty. Retrying in 5 seconds..."
    sleep 5
  fi
done