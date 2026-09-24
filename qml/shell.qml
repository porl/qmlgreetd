// The greeter shell: the shared bar, the login card, and the session menu, on
// Hyprland's layer shell. The card itself is still swappable (QMLGREETD_UI);
// the bar and menu are the shared qcommon components, so the login screen looks
// and behaves like the session.
//
// The bar's power block opens the same SessionMenu the session uses, but in the
// "greeter" context, so Lock and Log out are absent and suspend only appears
// where the hardware supports it. Power actions go through qmlgreetd's power
// command rather than the shell's systemctl calls.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

ShellRoot {
    id: shell

    readonly property Theme theme: Theme {}
    readonly property PowerCaps powerCaps: PowerCaps {}
    property Greeter greeter: Greeter {}

    Variants {
        model: Quickshell.screens

        delegate: Bar {
            required property ShellScreen modelData

            theme: shell.theme
            screen: modelData
            showWorkspaces: false
            onPowerRequested: sessionMenu.toggle()
        }
    }

    Variants {
        model: Quickshell.screens

        delegate: PanelWindow {
            required property ShellScreen modelData

            screen: modelData
            anchors {
                top: true
                left: true
                right: true
                bottom: true
            }
            color: "transparent"
            exclusionMode: ExclusionMode.Ignore
            WlrLayershell.layer: WlrLayer.Background
            // OnDemand, NOT Exclusive: Hyprland hit-tests exclusive-keyboard
            // layer surfaces before everything else ("forced above all") and
            // hands them the pointer even when it is not over them, so an
            // Exclusive fullscreen card swallows every click/scroll meant for
            // the bar. OnDemand still focuses the card on hover/click for the
            // password field.
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
            WlrLayershell.namespace: "quickshell-greeter"

            Loader {
                anchors.fill: parent
                source: shell.greeter.uiPath
                onLoaded: item.greeter = shell.greeter
            }
        }
    }

    SessionMenu {
        id: sessionMenu

        theme: shell.theme
        context: "greeter"
        canSuspend: shell.powerCaps.canSuspend
        canHibernate: shell.powerCaps.canHibernate
        onActionTriggered: action => shell.runAction(action)
    }

    // The bar's power block opens the menu; this exposes the same toggle to a
    // compositor keybind or script (`quickshell ipc call session toggle`).
    IpcHandler {
        target: "session"

        function toggle(): void {
            sessionMenu.toggle();
        }
    }

    function runAction(action: string): void {
        if (action === "suspend")
            greeter.power("suspend");
        else if (action === "hibernate")
            greeter.power("hibernate");
        else if (action === "reboot")
            greeter.power("reboot");
        else if (action === "poweroff")
            greeter.power("off");
    }
}
