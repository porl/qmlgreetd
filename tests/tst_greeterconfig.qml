import QtQuick
import QtTest
import "../qml/GreeterConfig.js" as Config

// The config resolution rules: file over environment over defaults, with each
// layer validated on its own so a bad value only falls through, never poisons.
TestCase {
    name: "GreeterConfig"

    function test_defaults_without_file_or_environment() {
        var r = Config.resolve({}, "");
        compare(r.invalid, false);
        compare(r.ui, "");
        compare(r.wallpaper, "");
        compare(r.wallpaperMode, "off");
        compare(r.wallpaperFps, 12);
        compare(r.wallpaperSeed, 1);
        compare(r.wallpaperMeteors, true);
        compare(r.wallpaperShowers, true);
        compare(r.wallpaperBuildings, true);
        compare(r.wallpaperMissiles, false);
        compare(r.wallpaperAntialias, true);
    }

    function test_file_values_apply() {
        var r = Config.resolve({}, JSON.stringify({
            ui: "/tmp/custom.qml",
            wallpaper: "/nix/store/x/nixos.png",
            wallpaperMode: "greeter",
            wallpaperFps: 6,
            wallpaperSeed: 0,
            wallpaperMeteors: false,
            wallpaperShowers: false,
            wallpaperBuildings: false,
            wallpaperMissiles: true,
            wallpaperAntialias: false
        }));
        compare(r.invalid, false);
        compare(r.ui, "/tmp/custom.qml");
        compare(r.wallpaper, "/nix/store/x/nixos.png");
        compare(r.wallpaperMode, "greeter");
        compare(r.wallpaperFps, 6);
        compare(r.wallpaperSeed, 0);
        compare(r.wallpaperMeteors, false);
        compare(r.wallpaperShowers, false);
        compare(r.wallpaperBuildings, false);
        compare(r.wallpaperMissiles, true);
        compare(r.wallpaperAntialias, false);
    }

    function test_file_wins_over_environment() {
        var env = {
            wallpaperMode: "always",
            wallpaperFps: "8",
            wallpaperMeteors: "1"
        };
        var r = Config.resolve(env, JSON.stringify({
            wallpaperMode: "idle",
            wallpaperFps: 6,
            wallpaperMeteors: false
        }));
        compare(r.wallpaperMode, "idle");
        compare(r.wallpaperFps, 6);
        compare(r.wallpaperMeteors, false);
    }

    function test_environment_fills_missing_file_keys() {
        var env = {
            ui: "/tmp/env.qml",
            wallpaperMode: "always",
            wallpaperMissiles: "1"
        };
        var r = Config.resolve(env, JSON.stringify({ wallpaperFps: 6 }));
        compare(r.ui, "/tmp/env.qml");
        compare(r.wallpaperMode, "always");
        compare(r.wallpaperMissiles, true);
        compare(r.wallpaperFps, 6);
    }

    function test_malformed_file_falls_back_to_environment() {
        var r = Config.resolve({ wallpaperMode: "always" }, "{not json");
        compare(r.invalid, true);
        compare(r.wallpaperMode, "always");
        compare(r.wallpaperFps, 12);
    }

    function test_non_object_json_is_invalid() {
        compare(Config.resolve({}, "[]").invalid, true);
        compare(Config.resolve({}, "3").invalid, true);
        compare(Config.resolve({}, "null").invalid, true);
    }

    function test_unknown_values_fall_through_per_layer() {
        var r = Config.resolve({ wallpaperMode: "nonsense", wallpaperFps: "x" }, JSON.stringify({
            wallpaperMode: "typo",
            wallpaperFps: "y",
            wallpaperSeed: "z"
        }));
        compare(r.wallpaperMode, "off");
        compare(r.wallpaperFps, 12);
        compare(r.wallpaperSeed, 1);

        var envWins = Config.resolve({ wallpaperMode: "always" }, JSON.stringify({ wallpaperMode: "typo" }));
        compare(envWins.wallpaperMode, "always");
    }

    function test_environment_booleans() {
        compare(Config.resolve({ wallpaperMeteors: "0" }, "").wallpaperMeteors, false);
        compare(Config.resolve({ wallpaperMeteors: "false" }, "").wallpaperMeteors, false);
        compare(Config.resolve({ wallpaperMeteors: "off" }, "").wallpaperMeteors, false);
        compare(Config.resolve({ wallpaperMeteors: "no" }, "").wallpaperMeteors, false);
        compare(Config.resolve({ wallpaperMissiles: "1" }, "").wallpaperMissiles, true);
        compare(Config.resolve({ wallpaperMeteors: "" }, "").wallpaperMeteors, true);
    }

    function test_numbers_parse_from_strings() {
        compare(Config.resolve({ wallpaperFps: "10" }, "").wallpaperFps, 10);
        compare(Config.resolve({}, JSON.stringify({ wallpaperFps: "4" })).wallpaperFps, 4);
        compare(Config.resolve({ wallpaperSeed: "-2" }, "").wallpaperSeed, -2);
    }

    function test_empty_strings_fall_back() {
        // An empty string in the file is "unset", so the environment still applies.
        compare(Config.resolve({ ui: "/tmp/env.qml" }, JSON.stringify({ ui: "" })).ui, "/tmp/env.qml");
        compare(Config.resolve({ wallpaper: "" }, "").wallpaper, "");
    }
}
