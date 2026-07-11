#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER="${SEALBOX_SERVER_BIN:-$ROOT_DIR/target/debug/sealbox-server}"
CLI="${SEALBOX_CLI_BIN:-$ROOT_DIR/target/debug/sealbox-cli}"

if [[ "${SEALBOX_SKIP_BUILD:-false}" != "true" || ! -x "$SERVER" || ! -x "$CLI" ]]; then
  cargo build --workspace --manifest-path "$ROOT_DIR/Cargo.toml"
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sealbox-multitenant.XXXXXX")"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

PORT="${SEALBOX_LIVE_PORT:-$(python3 - <<'PY'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)}"
URL="http://127.0.0.1:$PORT"
DB="$TMP_DIR/sealbox.db"
ROOT_TOKEN_FILE="$TMP_DIR/root-token"
printf '%s\n' "root-$(openssl rand -hex 32)" >"$ROOT_TOKEN_FILE"
chmod 600 "$ROOT_TOKEN_FILE"

start_server() {
  AUTH_TOKEN_FILE="$ROOT_TOKEN_FILE" \
  STORE_PATH="$DB" \
  LISTEN_ADDR="127.0.0.1:$PORT" \
    "$SERVER" >"$TMP_DIR/server.log" 2>&1 &
  SERVER_PID=$!
  for _ in $(seq 1 100); do
    if curl -fsS "$URL/healthz/ready" >/dev/null 2>&1; then
      return
    fi
    sleep 0.05
  done
  sed -n '1,200p' "$TMP_DIR/server.log" >&2
  echo "Sealbox did not become ready" >&2
  exit 1
}

stop_server() {
  kill "$SERVER_PID"
  wait "$SERVER_PID" 2>/dev/null || true
  SERVER_PID=""
}

root_cli() {
  SEALBOX_URL="$URL" \
  SEALBOX_TOKEN_FILE="$ROOT_TOKEN_FILE" \
  SEALBOX_OUTPUT_FORMAT=json \
    "$CLI" "$@"
}

tenant_cli() {
  local tenant_dir=$1
  shift
  SEALBOX_URL="$URL" \
  SEALBOX_API_VERSION=v2 \
  SEALBOX_TOKEN_FILE="$tenant_dir/token" \
  SEALBOX_PUBLIC_KEY_FILE="$tenant_dir/public.pem" \
  SEALBOX_PRIVATE_KEY_FILE="$tenant_dir/private.pem" \
  SEALBOX_OUTPUT_FORMAT=json \
    "$CLI" "$@"
}

json_value() {
  local expression=$1
  python3 -c 'import json,sys
raw=sys.stdin.read()
decoder=json.JSONDecoder()
value=None
for index,char in enumerate(raw):
    if char not in "[{":
        continue
    try:
        value,_=decoder.raw_decode(raw[index:])
        break
    except json.JSONDecodeError:
        pass
if value is None:
    raise SystemExit(f"no JSON value found in CLI output: {raw!r}")
result=eval(sys.argv[1], {"value": value})
print(json.dumps(result) if isinstance(result, (dict, list)) else result)' "$expression"
}

start_server

mkdir -p "$TMP_DIR/a" "$TMP_DIR/b"
create_a="$(root_cli tenant create --display-name "Tenant A" --token-label live --token-file "$TMP_DIR/a/token")"
create_b="$(root_cli tenant create --display-name "Tenant B" --token-label live --token-file "$TMP_DIR/b/token")"
tenant_a="$(printf '%s' "$create_a" | json_value 'value["tenant"]["id"]')"
tenant_b="$(printf '%s' "$create_b" | json_value 'value["tenant"]["id"]')"
token_a_id="$(printf '%s' "$create_a" | json_value 'value["token_metadata"]["id"]')"

"$CLI" key generate \
  --public-key-path "$TMP_DIR/a/public.pem" \
  --private-key-path "$TMP_DIR/a/private.pem" \
  --output json >/dev/null
"$CLI" key generate \
  --public-key-path "$TMP_DIR/b/public.pem" \
  --private-key-path "$TMP_DIR/b/private.pem" \
  --output json >/dev/null
tenant_cli "$TMP_DIR/a" key register >/dev/null
tenant_cli "$TMP_DIR/b" key register >/dev/null

printf '%s\n' 'a-shared-value' | tenant_cli "$TMP_DIR/a" secret set shared >/dev/null
printf '%s\n' 'b-shared-value' | tenant_cli "$TMP_DIR/b" secret set shared >/dev/null
printf '%s\n' 'a-only-value' | tenant_cli "$TMP_DIR/a" secret set a-only >/dev/null
printf '%s\n' 'b-only-value' | tenant_cli "$TMP_DIR/b" secret set b-only >/dev/null

value_a="$(tenant_cli "$TMP_DIR/a" secret get shared | json_value 'value["value"]')"
value_b="$(tenant_cli "$TMP_DIR/b" secret get shared | json_value 'value["value"]')"
[[ "$value_a" == "a-shared-value" ]]
[[ "$value_b" == "b-shared-value" ]]

list_a="$(tenant_cli "$TMP_DIR/a" secret list | json_value 'value')"
list_b="$(tenant_cli "$TMP_DIR/b" secret list | json_value 'value')"
printf '%s' "$list_a" | python3 -c 'import json,sys; keys={x["key"] for x in json.load(sys.stdin)}; assert keys == {"shared", "a-only"}, keys'
printf '%s' "$list_b" | python3 -c 'import json,sys; keys={x["key"] for x in json.load(sys.stdin)}; assert keys == {"shared", "b-only"}, keys'

if SEALBOX_URL="$URL" \
   SEALBOX_API_VERSION=v2 \
   SEALBOX_TOKEN_FILE="$TMP_DIR/a/token" \
   SEALBOX_PRIVATE_KEY_FILE="$TMP_DIR/b/private.pem" \
   SEALBOX_OUTPUT_FORMAT=json \
   "$CLI" secret get shared >/dev/null 2>&1; then
  echo "Tenant B private key unexpectedly decrypted Tenant A data" >&2
  exit 1
fi

root_status="$(curl -sS -o /dev/null -w '%{http_code}' \
  -H "Authorization: Bearer $(tr -d '\r\n' <"$ROOT_TOKEN_FILE")" \
  "$URL/v2/secrets")"
[[ "$root_status" == "401" ]]

stop_server
start_server
value_b_after_restart="$(tenant_cli "$TMP_DIR/b" secret get b-only | json_value 'value["value"]')"
[[ "$value_b_after_restart" == "b-only-value" ]]

root_cli tenant token revoke "$tenant_a" "$token_a_id" >/dev/null
if tenant_cli "$TMP_DIR/a" secret list >/dev/null 2>&1; then
  echo "Revoked Tenant A token was still accepted" >&2
  exit 1
fi
tenant_cli "$TMP_DIR/b" secret list >/dev/null

printf 'Sealbox multi-tenant live test passed: %s %s\n' "$tenant_a" "$tenant_b"
