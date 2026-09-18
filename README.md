# qmlgreetd

A customizable [Quickshell](https://quickshell.org) greeter for
[greetd](https://git.sr.ht/~kennylevinsen/greetd). It draws a minimal login card
over a blank compositor and speaks greetd's IPC protocol through a small Rust
transport process.

The project is a "blank greeter shell": the transport and auth logic are the
product, and the login/session screen is a swappable Quickshell component. The
bundled UI under `qml/example/` is an example, not the coupling point.

## Status

Implemented and tested: the greetd transport, auth state machine, enumeration,
`Exec` parsing, the fake backend, the example UI (user preselect, parallel
session choice, type-and-Enter login, power menu), remembered state, and a
watchdog. The default package bundles the binary, the QML tree, and a
`qmlgreetd-greeter` runner for a compositor. Not yet done: AccountsService
enrichment, suspend, the JSON config file, and real-host validation.

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

The UI opens fullscreen as a plain toplevel (`FloatingWindow`, not a
layer-shell `PanelWindow` — cage has no `wlr-layer-shell`). Press **Ctrl+Q** to
quit (which also tears down the fake backend).

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
