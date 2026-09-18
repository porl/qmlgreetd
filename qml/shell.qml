import QtQuick
import Quickshell

ShellRoot {
    id: shell

    property Greeter greeter: Greeter {}

    // cage has no wlr-layer-shell, so a plain FloatingWindow (PanelWindow needs it).
    FloatingWindow {
        id: window

        visible: true
        fullscreen: true
        color: shell.greeter.theme.base
        title: "qmlgreetd"

        Loader {
            id: ui

            anchors.fill: parent
            source: shell.greeter.uiPath

            onLoaded: item.greeter = shell.greeter
        }
    }
}
