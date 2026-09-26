import QtQuick
import Quickshell
import Quickshell.Io
import "GreeterConfig.js" as GreeterConfig

// The greeter's deployment config: /etc/qmlgreetd/config.json, read once at
// startup. A missing or malformed file must never cost a login, so the read is
// non-fatal and every value falls back to its QMLGREETD_* environment variable
// and then to a built-in default (see GreeterConfig.js for the precedence).
QtObject {
    id: config

    // QMLGREETD_CONFIG points a development run at its own file.
    readonly property string path: Quickshell.env("QMLGREETD_CONFIG") || "/etc/qmlgreetd/config.json"

    // A declared property, not a child: QtObject has no default property to
    // hold children.
    property FileView file: FileView {
        path: config.path
        blockLoading: true
        printErrors: false
    }

    readonly property var values: {
        var resolved = GreeterConfig.resolve({
            ui: Quickshell.env("QMLGREETD_UI"),
            wallpaper: Quickshell.env("QMLGREETD_WALLPAPER"),
            wallpaperMode: Quickshell.env("QMLGREETD_WALLPAPER_MODE"),
            wallpaperFps: Quickshell.env("QMLGREETD_WALLPAPER_FPS"),
            wallpaperSeed: Quickshell.env("QMLGREETD_WALLPAPER_SEED"),
            wallpaperMeteors: Quickshell.env("QMLGREETD_WALLPAPER_METEORS"),
            wallpaperShowers: Quickshell.env("QMLGREETD_WALLPAPER_SHOWERS"),
            wallpaperBuildings: Quickshell.env("QMLGREETD_WALLPAPER_BUILDINGS"),
            wallpaperMissiles: Quickshell.env("QMLGREETD_WALLPAPER_MISSILES"),
            wallpaperAntialias: Quickshell.env("QMLGREETD_WALLPAPER_ANTIALIAS")
        }, file.text());
        if (resolved.invalid)
            console.warn("qmlgreetd: ignoring malformed config file " + config.path);
        return resolved;
    }

    readonly property string ui: values.ui
    readonly property string wallpaper: values.wallpaper
    readonly property string wallpaperMode: values.wallpaperMode
    readonly property int wallpaperFps: values.wallpaperFps
    readonly property int wallpaperSeed: values.wallpaperSeed
    readonly property bool wallpaperMeteors: values.wallpaperMeteors
    readonly property bool wallpaperShowers: values.wallpaperShowers
    readonly property bool wallpaperBuildings: values.wallpaperBuildings
    readonly property bool wallpaperMissiles: values.wallpaperMissiles
    readonly property bool wallpaperAntialias: values.wallpaperAntialias
}
