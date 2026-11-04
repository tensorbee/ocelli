#!/bin/sh
# Start the frame-open helper, then the example viewer. The helper prints
# only its base URL. This script repeats that URL on stderr so the operator
# can see it beside Vite.
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
cd "$root"
cargo build -p ocelli-example-server
url_file=$(mktemp)
pid=""
cleanup() {
  if [ -n "$pid" ]; then
    kill "$pid" 2>/dev/null || true
  fi
  rm -f "$url_file"
}
trap cleanup EXIT INT TERM
"$root/target/debug/ocelli-example-server" >"$url_file" &
pid=$!
i=0
while [ ! -s "$url_file" ]; do
  i=$((i + 1))
  if [ "$i" -gt 100 ]; then
    echo "the example helper did not print a URL" >&2
    exit 1
  fi
  sleep 0.1
done
url=$(head -n 1 "$url_file")
echo "example helper $url" >&2
export VITE_EXAMPLE_SERVER="$url"
# Stay in this shell so the EXIT trap stops the helper when Vite stops.
npm run dev -w @ocelli/example-viewer
