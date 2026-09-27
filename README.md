# qmlgreetd

A customizable [Quickshell](https://quickshell.org) greeter for
[greetd](https://git.sr.ht/~kennylevinsen/greetd). It draws a login card over a
Hyprland layer shell and speaks greetd's IPC protocol through a small Rust
transport process.

The project is a "blank greeter shell": the transport and auth logic are the
product, and the login/session screen is a swappable Quickshell component. The
bundled UI (`qml/LoginScreen.qml`) is an example, not the coupling point.

## Status

Implemented and tested: the greetd transport, auth state machine, enumeration
(including AccountsService real names and avatars when the daemon is running),
`Exec` parsing, the fake backend, the bundled UI (user preselect, parallel
session choice, type-and-Enter login, remembered state), a watchdog and the
JSON config file. The bundled UI runs on Hyprland and shares the
[qcommon](../qcommon) bar, popouts and session menu with the session shell, so
the login screen has the same power block, network/brightness popouts and
capability-aware session menu. Power actions go through `qmlgreetd power` (`off`,
`reboot`, `suspend`, `hibernate`; gated by `QMLGREETD_POWER`). The default
package bundles the binary, the merged QML tree, and a `qmlgreetd-greeter`
runner. Not yet done: real-host validation.

## Configuration

The greeter reads `/etc/qmlgreetd/config.json` once at startup. It is the
deployment's look; every key also has a `QMLGREETD_*` environment fallback and a
built-in default, so a missing (or malformed) file never costs a login. The file
wins over the environment.

| key | environment fallback | default | meaning |
|---|---|---|---|
| `ui` | `QMLGREETD_UI` | bundled `LoginScreen.qml` | custom login screen |
| `wallpaper` | `QMLGREETD_WALLPAPER` | none | static background image |
| `wallpaperMode` | `QMLGREETD_WALLPAPER_MODE` | `off` | night sky: `off`, `greeter`, `idle`, `always` |
| `wallpaperFps` | `QMLGREETD_WALLPAPER_FPS` | `12` | night-sky animation clock |
| `wallpaperSeed` | `QMLGREETD_WALLPAPER_SEED` | `1` | skyline seed |
| `wallpaperMeteors` | `QMLGREETD_WALLPAPER_METEORS` | `true` | meteors |
| `wallpaperShowers` | `QMLGREETD_WALLPAPER_SHOWERS` | `true` | occasional meteor showers |
| `wallpaperBuildings` | `QMLGREETD_WALLPAPER_BUILDINGS` | `true` | the city |
| `wallpaperMissiles` | `QMLGREETD_WALLPAPER_MISSILES` | `false` | missile command |
| `wallpaperAntialias` | `QMLGREETD_WALLPAPER_ANTIALIAS` | `true` | antialiased sky dots |
| `theme` | `QMLGREETD_THEME` | qcommon's Mocha roles | hex colour overrides |

`theme` is an object of colour overrides for the roles qcommon's `Theme.qml`
defines (`base`, `backdrop`, `surface`, `surfaceAlt`, `border`, `bar`, `text`,
`subtext`, `overlay`, `accent`, `danger`, `skyTop`, `skyBottom`, `buildingGlow`,
`starGlow`, `moonGlow`, `missileTrail`, `explosionGlow`). Only `#rgb`,
`#rrggbb` and `#aarrggbb` values are accepted, validated per layer, so a typo
falls through to the environment and then to the role's default; a deployment
only needs to name the roles it changes. `QMLGREETD_THEME` takes the same block
as a JSON string.

`QMLGREETD_CONFIG` points the greeter at a different file (development runs):

```json
{
  "wallpaperMode": "greeter",
  "wallpaperFps": 6,
  "theme": { "accent": "#f38ba8" }
}
```

Power actions are deliberately not part of the config: `QMLGREETD_POWER` is a
capability the runner grants, not a look.

## AccountsService

The login users come from `/etc/passwd` (UID ≥ `UID_MIN`, with a login shell).
When `accountsservice` is installed and running, each user's real name and
avatar are overlaid from it, by username; without the daemon the greeter just
shows the passwd data. Avatars are read from the icon file AccountsService
reports, and are dropped when that file does not exist or cannot be read. That
default is `$HOME/.face`, which a greeter running as its own user cannot reach
when home directories are mode 0700 (the NixOS default), so on such hosts put
the image in `/var/lib/AccountsService/icons/<username>` and set the user's
`Icon=` keyfile entry to it. A user with no avatar shows the first letter of
their display name instead.

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
