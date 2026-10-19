#!/usr/bin/env bash
# pact-broker.sh start [port] | stop — runs the Pact Broker the differential harness's broker
# fixtures are pinned to (staged plan S1.3, tools/verifier-diff/pact-broker.json), and prints its URL.
#
# The image is started by its pinned digest, so the ledger is always judged against the broker it was
# kept against. It keeps its database in SQLite inside the container: stopping it forgets everything,
# and nothing the harness does needs it to remember. Moving the pin is a change to that file, not to
# this script.
#
# PACT_BROKER_IMAGE starts another image instead (the weekly `Latest Pact Broker` workflow passes
# pactfoundation/pact-broker:latest), and PACT_BROKER_CONTAINER names the container, so a second
# broker can run beside the pinned one.
set -euo pipefail

name="${PACT_BROKER_CONTAINER:-verifier-diff-broker}"
pin="$(git rev-parse --show-toplevel)/tools/verifier-diff/pact-broker.json"

case "${1:-}" in
  start)
    port="${2:-9292}"
    image="${PACT_BROKER_IMAGE:-$(jq -r '"\(.image)@\(.digest)"' "$pin")}"
    docker pull --quiet "$image" >&2
    docker rm -f "$name" >/dev/null 2>&1 || true
    docker run -d --name "$name" -p "127.0.0.1:$port:9292" \
      -e PACT_BROKER_DATABASE_URL=sqlite:////tmp/pact_broker.sqlite3 \
      -e PACT_BROKER_PORT=9292 \
      "$image" >&2
    url="http://127.0.0.1:$port"
    for _ in $(seq 1 120); do
      if curl -fsS -o /dev/null "$url/diagnostic/status/heartbeat"; then
        echo "$url"
        exit 0
      fi
      sleep 1
    done
    echo "pact-broker.sh: the broker did not answer at $url within two minutes" >&2
    docker logs "$name" >&2 || true
    exit 2
    ;;
  stop)
    docker rm -f "$name" >/dev/null
    ;;
  *)
    echo "usage: pact-broker.sh start [port] | stop" >&2
    exit 2
    ;;
esac
