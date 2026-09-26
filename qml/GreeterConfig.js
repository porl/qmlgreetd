.pragma library

// Resolves the greeter's look from the config file and the environment, with
// no QML types in it so qmltestrunner can pin the rules. The file wins over
// its environment fallback and both win over the built-in default: the
// deployed config is authoritative, QMLGREETD_* is the per-run escape hatch.

var DEFAULTS = {
    ui: "",
    wallpaper: "",
    wallpaperMode: "off",
    wallpaperFps: 12,
    wallpaperSeed: 1,
    wallpaperMeteors: true,
    wallpaperShowers: true,
    wallpaperBuildings: true,
    wallpaperMissiles: false,
    wallpaperAntialias: true
};

// The night-sky modes (NightSkySim.js); anything else in the config would
// otherwise fall through to "always".
var MODES = [ "always", "idle", "greeter", "off" ];

// The parsed file as an object, or null when the text is not a JSON object.
// Empty text is an absent file, not an error.
function parse(text) {
    if (!text)
        return {};
    var value;
    try {
        value = JSON.parse(text);
    } catch (error) {
        return null;
    }
    if (!value || typeof value !== "object" || Array.isArray(value))
        return null;
    return value;
}

function stringOf(value, fallback) {
    return typeof value === "string" && value.length > 0 ? value : fallback;
}

function intOf(value, fallback) {
    var parsed = parseInt(value, 10);
    return isNaN(parsed) ? fallback : parsed;
}

function boolOf(value, fallback) {
    if (typeof value === "boolean")
        return value;
    if (typeof value === "number")
        return value !== 0;
    if (typeof value === "string" && value.length > 0) {
        var lowered = value.toLowerCase();
        return lowered !== "0" && lowered !== "false" && lowered !== "off" && lowered !== "no";
    }
    return fallback;
}

function modeOf(value, fallback) {
    return typeof value === "string" && MODES.indexOf(value) !== -1 ? value : fallback;
}

// File value, else environment value, else the default. Each layer is
// validated on its own, so an invalid file value falls through to the
// environment rather than poisoning the result.
function pick(convert, envValue, fileValue, fallback) {
    return convert(fileValue, convert(envValue, fallback));
}

function resolve(env, text) {
    env = env || {};
    var parsed = parse(text);
    var file = parsed || {};

    return {
        invalid: parsed === null,
        ui: pick(stringOf, env.ui, file.ui, DEFAULTS.ui),
        wallpaper: pick(stringOf, env.wallpaper, file.wallpaper, DEFAULTS.wallpaper),
        wallpaperMode: pick(modeOf, env.wallpaperMode, file.wallpaperMode, DEFAULTS.wallpaperMode),
        wallpaperFps: pick(intOf, env.wallpaperFps, file.wallpaperFps, DEFAULTS.wallpaperFps),
        wallpaperSeed: pick(intOf, env.wallpaperSeed, file.wallpaperSeed, DEFAULTS.wallpaperSeed),
        wallpaperMeteors: pick(boolOf, env.wallpaperMeteors, file.wallpaperMeteors, DEFAULTS.wallpaperMeteors),
        wallpaperShowers: pick(boolOf, env.wallpaperShowers, file.wallpaperShowers, DEFAULTS.wallpaperShowers),
        wallpaperBuildings: pick(boolOf, env.wallpaperBuildings, file.wallpaperBuildings, DEFAULTS.wallpaperBuildings),
        wallpaperMissiles: pick(boolOf, env.wallpaperMissiles, file.wallpaperMissiles, DEFAULTS.wallpaperMissiles),
        wallpaperAntialias: pick(boolOf, env.wallpaperAntialias, file.wallpaperAntialias, DEFAULTS.wallpaperAntialias)
    };
}
