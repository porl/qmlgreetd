{
  description = "qmlgreetd - a customizable Quickshell greeter for greetd";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  inputs.qcommon = {
    url = "github:porl/qcommon";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { self, nixpkgs, qcommon }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems =
        f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      qmlgreetd =
        pkgs:
        pkgs.rustPlatform.buildRustPackage {
          pname = "qmlgreetd";
          version = "0.1.0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            description = "A customizable Quickshell greeter for greetd";
            license = nixpkgs.lib.licenses.gpl3Plus;
            mainProgram = "qmlgreetd";
            platforms = systems;
          };
        };

      # Shared qcommon components are copied in first, then the greeter's own
      # QML, so both resolve by name through QML's implicit directory import.
      qmlTree =
        pkgs:
        pkgs.runCommand "qmlgreetd-qml" { } ''
          mkdir -p $out/share/qmlgreetd/qml
          cp -r ${qcommon.packages.${pkgs.stdenv.hostPlatform.system}.default}/share/qcommon/qml/. $out/share/qmlgreetd/qml/
          cp -r ${./qml}/. $out/share/qmlgreetd/qml/
        '';

      greeter =
        pkgs:
        pkgs.writeShellScriptBin "qmlgreetd-greeter" ''
          export QMLGREETD_BIN="${qmlgreetd pkgs}/bin/qmlgreetd"
          # cage has no decorations; without this Qt adds a titlebar.
          export QT_WAYLAND_DISABLE_WINDOWDECORATION=1
          exec "${pkgs.quickshell}/bin/quickshell" --path "${qmlTree pkgs}/share/qmlgreetd/qml" "$@"
        '';

      package =
        pkgs:
        pkgs.symlinkJoin {
          name = "qmlgreetd-0.1.0";
          paths = [
            (qmlgreetd pkgs)
            (qmlTree pkgs)
            (greeter pkgs)
          ];
        };
    in
    {
      packages = forAllSystems (pkgs: {
        default = package pkgs;
      });

      checks = forAllSystems (pkgs: {
        build = qmlgreetd pkgs;

        clippy = pkgs.rustPlatform.buildRustPackage {
          pname = "qmlgreetd-clippy";
          version = "0.1.0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.clippy ];
          doCheck = false;
          buildPhase = "cargo clippy --all-targets -- -D warnings";
          installPhase = "touch $out";
        };

        fmt = pkgs.runCommand "qmlgreetd-fmt-check" {
          nativeBuildInputs = [
            pkgs.cargo
            pkgs.rustfmt
          ];
        } ''
          cp -r ${self} src
          chmod -R u+w src
          cd src
          export CARGO_HOME=$TMPDIR/cargo-home
          cargo fmt --check
          touch $out
        '';
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.clippy
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.quickshell
          ];
        };
      });
    };
}
