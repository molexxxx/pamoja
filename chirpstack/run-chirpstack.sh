#!/usr/bin/env bash
# Bring a ChirpStack network server up and run the pamoja-gateway interop test against it.
#
# Run from the repo root as `cargo xtask chirpstack`, or directly as
# `bash chirpstack/run-chirpstack.sh`. The stack runs under docker compose while the test runs
# here, because the test needs cargo and the stack is six containers rather than one image.
#
# Everything this creates is thrown away on exit: the stack, its volumes, and the API key.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/.." && pwd)"
COMPOSE="docker compose -f $HERE/docker-compose.yml"
REST="http://localhost:8090"

# The identifiers this run uses. The device and application identifiers are one repeated byte
# each, which sidesteps the one real trap here: LoRaWAN sends an EUI least significant byte
# first, while an API renders it most significant byte first, and a value that reads the same
# either way cannot be registered backwards.
GATEWAY_EUI="0102030405060708"
DEV_EUI="1111111111111111"
JOIN_EUI="2222222222222222"
APP_KEY="33333333333333333333333333333333"

PYTHON=""
for candidate in python3 python; do
    if command -v "$candidate" >/dev/null 2>&1; then
        PYTHON="$candidate"
        break
    fi
done
if [ -z "$PYTHON" ]; then
    echo "python is required to read the API responses"
    exit 1
fi

cleanup() {
    echo
    echo "===== chirpstack log tail ====="
    $COMPOSE logs --tail=30 chirpstack 2>/dev/null || true
    echo "===== gateway bridge log tail ====="
    $COMPOSE logs --tail=20 chirpstack-gateway-bridge 2>/dev/null || true
    $COMPOSE down -v --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Read one field out of a JSON response, given a Python subscript such as "['result'][0]['id']".
field() {
    "$PYTHON" -c "import json,sys; print(json.load(sys.stdin)$1)"
}

# Call the gRPC to REST proxy as the API key.
api() {
    local method="$1"
    local path="$2"
    local body="${3:-}"
    if [ -n "$body" ]; then
        curl -sS -X "$method" "$REST$path" \
            -H "Grpc-Metadata-Authorization: Bearer $TOKEN" \
            -H "Content-Type: application/json" \
            -d "$body"
    else
        curl -sS -X "$method" "$REST$path" \
            -H "Grpc-Metadata-Authorization: Bearer $TOKEN"
    fi
}

echo "chirpstack: bringing the stack up"
if ! $COMPOSE up -d; then
    echo "chirpstack: the stack did not start"
    exit 1
fi

# ChirpStack migrates its database on first boot, so the first calls fail until it is ready.
# Minting the API key is the readiness check as well as the credential: it needs the server and
# the database, and there is no way to seed a key from configuration.
echo "chirpstack: waiting for the server and minting an API key"
TOKEN=""
# The command reads every file in the configuration directory itself. Docker Desktop on
# Windows serves that bind mount in a way this second process cannot read, though the server
# that already started from it is fine, so the files are copied inside the container first.
# On Linux, where CI runs, the copy is simply a copy.
for attempt in $(seq 1 60); do
    OUTPUT=$($COMPOSE exec -T chirpstack sh -c "mkdir -p /tmp/cfg && cp /etc/chirpstack/*.toml /tmp/cfg/ 2>/dev/null && chirpstack --config /tmp/cfg create-api-key --name pamoja-interop-$attempt" 2>/dev/null)
    TOKEN=$(printf '%s\n' "$OUTPUT" | sed -n 's/^[[:space:]]*token:[[:space:]]*//p' | tr -d '\r')
    if [ -n "$TOKEN" ]; then
        break
    fi
    sleep 2
done
if [ -z "$TOKEN" ]; then
    echo "chirpstack: the server never became ready; the containers are:"
    $COMPOSE ps
    exit 1
fi

echo "chirpstack: waiting for the REST proxy"
for _ in $(seq 1 30); do
    if curl -sf -o /dev/null -H "Grpc-Metadata-Authorization: Bearer $TOKEN" \
        "$REST/api/tenants?limit=1"; then
        break
    fi
    sleep 2
done

# A fresh stack creates one tenant. Its identifier is widely quoted as a fixed value, but
# ChirpStack has an open issue saying it is not unique, so read it rather than assume it.
TENANT_ID=$(api GET "/api/tenants?limit=1" | field "['result'][0]['id']")
if [ -z "$TENANT_ID" ]; then
    echo "chirpstack: no tenant to register against"
    exit 1
fi
echo "chirpstack: tenant $TENANT_ID"

echo "chirpstack: registering the gateway"
api POST "/api/gateways" "{\"gateway\":{\"gatewayId\":\"$GATEWAY_EUI\",\"name\":\"pamoja-interop\",\"description\":\"pamoja-gateway interop test\",\"tenantId\":\"$TENANT_ID\"}}" >/dev/null

echo "chirpstack: registering the application and the device"
APPLICATION_ID=$(api POST "/api/applications" \
    "{\"application\":{\"name\":\"pamoja-interop\",\"description\":\"pamoja-gateway interop test\",\"tenantId\":\"$TENANT_ID\"}}" |
    field "['id']")

PROFILE_ID=$(api POST "/api/device-profiles" \
    "{\"deviceProfile\":{\"name\":\"pamoja-otaa\",\"tenantId\":\"$TENANT_ID\",\"region\":\"EU868\",\"macVersion\":\"LORAWAN_1_0_3\",\"regParamsRevision\":\"RP002_1_0_5\",\"supportsOtaa\":true,\"uplinkInterval\":3600}}" |
    field "['id']")

api POST "/api/devices" \
    "{\"device\":{\"devEui\":\"$DEV_EUI\",\"joinEui\":\"$JOIN_EUI\",\"name\":\"pamoja-node\",\"applicationId\":\"$APPLICATION_ID\",\"deviceProfileId\":\"$PROFILE_ID\"}}" >/dev/null

# A LoRaWAN 1.0.x device keeps its root key in nwkKey; appKey is the 1.1 field. Putting it in the
# wrong one makes the join fail with no error anywhere.
api POST "/api/devices/$DEV_EUI/keys" \
    "{\"deviceKeys\":{\"devEui\":\"$DEV_EUI\",\"nwkKey\":\"$APP_KEY\"}}" >/dev/null

echo "chirpstack: application $APPLICATION_ID, device $DEV_EUI"

cd "$REPO" || exit 1
export PAMOJA_CHIRPSTACK_UDP="127.0.0.1:1700"
export PAMOJA_CHIRPSTACK_MQTT="127.0.0.1:1883"
export PAMOJA_CHIRPSTACK_GATEWAY_EUI="$GATEWAY_EUI"
export PAMOJA_CHIRPSTACK_DEV_EUI="$DEV_EUI"
export PAMOJA_CHIRPSTACK_JOIN_EUI="$JOIN_EUI"
export PAMOJA_CHIRPSTACK_APP_KEY="$APP_KEY"
export PAMOJA_CHIRPSTACK_APPLICATION_ID="$APPLICATION_ID"

echo
echo "chirpstack: running the interop test"
cargo test -p pamoja-examples --test chirpstack -- --ignored --nocapture
exit $?
