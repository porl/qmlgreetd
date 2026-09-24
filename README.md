# qmlgreetd

A customizable [Quickshell](https://quickshell.org) greeter for
[greetd](https://git.sr.ht/~kennylevinsen/greetd). It draws a login card over a
Hyprland layer shell and speaks greetd's IPC protocol through a small Rust
transport process.

The project is a "blank greeter shell": the transport and auth logic are the
product, and the login/session screen is a swappable Quickshell component. The
bundled UI (`qml/LoginScreen.qml`) is an example, not the coupling point.

## Status

Implemented and tested: the greetd transport, auth state machine, enumeration,
`Exec` parsing, the fake backend, the bundled UI (user preselect, parallel
session choice, type-and-Enter login, remembered state) and a watchdog. The
bundled UI runs on Hyprland and shares the [qcommon](../qcommon) bar, popouts
and session menu with the session shell, so the login screen has the same power
block, network/brightness popouts and capability-aware session menu. Power
actions go through `qmlgreetd power` (`off`, `reboot`, `suspend`, `hibernate`;
gated by `QMLGREETD_POWER`). The default package bundles the binary, the merged
QML tree, and a `qmlgreetd-greeter` runner. Not yet done: AccountsService
enrichment, the JSON config file, and real-host validation.

## Try the UI against the fake backend

This runs the example screen in your current Wayland session, talking to the
scripted fake greetd backend. It never touches a real greetd.

```
nix develop
./scripts/dev-greeter.sh                 # scenario "normal", password "hunter2"
./scripts/dev-greeter.sh info            # an informational message first
./scripts/dev-greeter.sh auth-error      # every password is rejected
```

The host-provided `quickshell` on some systems (e.g. a `noctalia-qs` fork) may be
broken or incompatible; `nix develop` puts a known-good `quickshell` 0.3.0 on
`PATH`, and the launcher uses that.

The UI opens fullscreen on Hyprland's layer shell (a background
`PanelWindow` for the card, a `PanelWindow` bar on top, and the shared session
menu on the overlay layer). Press **Ctrl+Q** to quit (which also tears down the
fake backend).

On a non-NixOS host the nix-built Quickshell cannot see the host EGL/GPU stack
and fails with `EGL not available`. Lend it the host drivers with `nixGL`:

```
QMLGREETD_QS_WRAPPER="nix run --impure github:nix-community/nixGL --" \
    ./scripts/dev-greeter.sh
```

## Building

```
nix build
nix flake check
```

For local iteration:

```
nix develop
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

GPL-3.0-or-later. See `LICENSE`.
