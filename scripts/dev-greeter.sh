#!/usr/bin/env bash
# Run the example greeter UI against the fake greetd backend, in the current
# Wayland session. Development only: this never touches a real greetd.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scenario="${1:-normal}"
password="${2:-hunter2}"

quickshell="${QMLGREETD_QS:-quickshell}"
if ! command -v "$quickshell" >/dev/null 2>&1; then
    echo "dev-greeter: cannot find '$quickshell' on PATH." >&2
    echo "dev-greeter: run this inside 'nix develop', or set QMLGREETD_QS." >&2
    exit 1
fi

# Optional wrapper to lend the host GPU/EGL stack to a nix-built quickshell on
# non-NixOS hosts, e.g. QMLGREETD_QS_WRAPPER="nix run --impure github:nix-community/nixGL --"
read -r -a qs_wrapper <<< "${QMLGREETD_QS_WRAPPER:-}"

if [[ -n "${QMLGREETD_BIN:-}" ]]; then
    binary="$QMLGREETD_BIN"
else
    (cd "$repo" && cargo build --quiet)
    binary="$repo/target/debug/qmlgreetd"
fi

runtime="$(mktemp -d "${TMPDIR:-/tmp}/qmlgreetd-dev.XXXXXX")"
socket="$runtime/greetd.sock"
mock_log="$runtime/mock.log"
mock_pid=""

# Stable across runs so remembered user/session survives a restart.
state="${QMLGREETD_STATE:-$repo/.dev/state.json}"
mkdir -p "$(dirname "$state")"

cleanup() {
    if [[ -n "$mock_pid" ]] && kill -0 "$mock_pid" 2>/dev/null; then
        cmdline="$(tr '\0' ' ' < "/proc/$mock_pid/cmdline" 2>/dev/null || true)"
        if [[ "$cmdline" == *mock-greetd* ]]; then
            kill "$mock_pid" 2>/dev/null || true
        fi
    fi
    if [[ -n "$mock_pid" ]]; then
        wait "$mock_pid" 2>/dev/null || true
    fi
    rm -rf "$runtime"
}
trap cleanup EXIT INT TERM

env -u GREETD_SOCK QMLGREETD_MOCK_PASSWORD="$password" \
    "$binary" mock-greetd --socket "$socket" --scenario "$scenario" >"$mock_log" 2>&1 &
mock_pid=$!

for _ in $(seq 1 100); do
    [[ -S "$socket" ]] && break
    sleep 0.05
done
if [[ ! -S "$socket" ]]; then
    echo "dev-greeter: the mock greetd socket never appeared; see $mock_log" >&2
    exit 1
fi

echo "dev-greeter: scenario=$scenario password=$password socket=$socket" >&2
echo "dev-greeter: stop the UI to tear the mock down" >&2

if [[ -z "${QMLGREETD_QS_WRAPPER:-}" && ! -e /run/opengl-driver ]]; then
    echo "dev-greeter: no /run/opengl-driver (non-NixOS?); if quickshell fails with" >&2
    echo "dev-greeter: 'EGL not available', retry with:" >&2
    echo "dev-greeter:   QMLGREETD_QS_WRAPPER='nix run --impure github:nix-community/nixGL --' $0" >&2
fi

# The shared components live in qcommon; merge the two trees so the QML
# resolves by name, matching the packaged tree.
qcommon="${QMLGREETD_COMMON:-$repo/../qcommon/qml}"
merged="$(mktemp -d)"
cleanup_qml() { rm -rf "$merged"; }
trap 'cleanup_qml; cleanup' EXIT INT TERM
cp -r "$qcommon"/. "$merged"/
cp -r "$repo/qml"/. "$merged"/

GREETD_SOCK="$socket" \
    QMLGREETD_BIN="$binary" \
    QMLGREETD_MOCK=1 \
    QMLGREETD_POWER=mock \
    QMLGREETD_STATE="$state" \
    QT_WAYLAND_DISABLE_WINDOWDECORATION=1 \
    "${qs_wrapper[@]}" "$quickshell" --path "$merged"
