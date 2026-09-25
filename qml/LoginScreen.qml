// The login card, drawn over the wallpaper. The host (shell.qml) supplies the
// bar, the clock and the session menu; this file is only the card, so it stays
// swappable via QMLGREETD_UI.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Io

FocusScope {
    id: screen

    property var greeter: null

    readonly property var theme: greeter ? greeter.theme : null
    readonly property color cBase: theme ? theme.base : "#000000"
    readonly property color cSurface: theme ? theme.surface : "#e6000000"
    readonly property color cSurfaceAlt: theme ? theme.surfaceAlt : "#45475a"
    readonly property color cBorder: theme ? theme.border : "#6c7086"
    readonly property color cOverlay: theme ? theme.overlay : "#6c7086"
    readonly property color cSubtext: theme ? theme.subtext : "#bac2de"
    readonly property color cText: theme ? theme.text : "#cdd6f4"
    readonly property color cAccent: theme ? theme.accent : "#89b4fa"
    readonly property color cDanger: theme ? theme.danger : "#f38ba8"
    readonly property int cRadius: theme ? theme.radius : 16
    readonly property string cFont: theme ? theme.fontFamily : "sans-serif"
    readonly property int cFontSize: theme ? theme.fontSize : 16
    readonly property int cFontSmall: theme ? theme.fontSizeSmall : 14
    readonly property int cFontTiny: theme ? theme.fontSizeTiny : 12

    readonly property string cWallpaper: greeter ? greeter.wallpaper : ""

    readonly property bool hasUser: greeter !== null && greeter.chosenUser.length > 0
    readonly property bool choosingUser: greeter !== null && greeter.usersExpanded
    readonly property bool choosingSession: greeter !== null && greeter.sessionsExpanded
    readonly property bool enteringPassword: greeter !== null && greeter.stage === "login" && hasUser && !choosingUser && !choosingSession
    readonly property bool enteringPrompt: greeter !== null && greeter.stage === "prompt"
    readonly property bool busy: greeter !== null && greeter.busy

    property int busyDots: 1
    readonly property string busyLabel: "Signing in" + [".", "..", "..."][(busyDots - 1) % 3]

    focus: true

    onEnteringPasswordChanged: if (enteringPassword) passwordField.forceActiveFocus()
    onEnteringPromptChanged: if (enteringPrompt) promptField.forceActiveFocus()
    onBusyChanged: {
        if (busy)
            return;
        if (enteringPassword)
            passwordField.forceActiveFocus();
        else if (enteringPrompt)
            promptField.forceActiveFocus();
    }
    Component.onCompleted: if (enteringPassword) passwordField.forceActiveFocus()

    Timer {
        running: screen.busy
        interval: 400
        repeat: true
        onTriggered: screen.busyDots = screen.busyDots % 3 + 1
    }

    Image {
        anchors.fill: parent
        visible: screen.cWallpaper !== "" && !(greeter && greeter.nightSkyActive)
        source: screen.cWallpaper
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
    }

    Rectangle {
        anchors.fill: parent
        visible: screen.cWallpaper === "" && !(greeter && greeter.nightSkyActive)
        color: screen.cBase
    }

    Rectangle {
        id: card

        anchors.centerIn: parent
        width: 360
        height: layout.implicitHeight + 40
        radius: screen.cRadius
        color: screen.cSurface
        border.width: 1
        border.color: screen.cBorder

        ColumnLayout {
            id: layout

            anchors {
                fill: parent
                margins: 20
            }

            spacing: 12

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    visible: screen.hasUser && !screen.choosingUser && greeter.stage !== "finished"
                    text: "←"
                    color: backArea.containsMouse ? screen.cText : screen.cOverlay
                    font.family: screen.cFont
                    font.pixelSize: screen.cFontSize + 2

                    MouseArea {
                        id: backArea
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        onClicked: greeter.usersExpanded = true
                    }
                }

                Text {
                    Layout.fillWidth: true
                    color: screen.cText
                    font.family: screen.cFont
                    font.pixelSize: screen.cFontSize + 4
                    font.bold: true
                    elide: Text.ElideRight
                    text: {
                        if (!greeter)
                            return "";
                        if (greeter.stage === "finished")
                            return "Starting session";
                        if (greeter.chosenUserObject)
                            return greeter.chosenUserObject.display_name || greeter.chosenUserObject.username;
                        return "Sign in";
                    }
                }
            }

            ListView {
                id: userList

                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(5, Math.max(1, greeter ? greeter.users.length : 1)) * 40
                visible: screen.choosingUser || !screen.hasUser
                clip: true
                focus: visible
                model: greeter ? greeter.users : []
                currentIndex: greeter ? greeter.userCursor : -1

                onVisibleChanged: if (visible) forceActiveFocus()

                delegate: Rectangle {
                    required property var modelData
                    required property int index

                    width: ListView.view.width
                    height: 40
                    radius: 8
                    color: index === (greeter ? greeter.userCursor : -1) ? screen.cSurfaceAlt : "transparent"

                    Text {
                        anchors {
                            left: parent.left
                            leftMargin: 12
                            verticalCenter: parent.verticalCenter
                        }

                        color: screen.cText
                        font.family: screen.cFont
                        font.pixelSize: screen.cFontSmall
                        text: modelData.display_name || modelData.username
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: greeter.chooseUser(modelData.username)
                    }
                }

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Down) {
                        greeter.userCursor = Math.min(greeter.users.length - 1, greeter.userCursor + 1);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Up) {
                        greeter.userCursor = Math.max(0, greeter.userCursor - 1);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        if (greeter.users.length > 0)
                            greeter.chooseUser(greeter.users[greeter.userCursor].username);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Escape && screen.hasUser) {
                        greeter.usersExpanded = false;
                        event.accepted = true;
                    }
                }
            }

            TextField {
                id: passwordField

                Layout.fillWidth: true
                Layout.preferredHeight: 40
                visible: screen.enteringPassword
                enabled: !screen.busy
                opacity: screen.busy ? 0.5 : 1.0
                color: screen.cText
                font.family: screen.cFont
                font.pixelSize: screen.cFontSmall
                echoMode: TextInput.Password
                placeholderText: "Password"
                placeholderTextColor: screen.cOverlay
                leftPadding: 12
                rightPadding: 12
                background: Rectangle {
                    radius: 8
                    color: screen.cSurfaceAlt
                    border.width: 1
                    border.color: passwordField.activeFocus ? screen.cAccent : screen.cBorder
                }
                onVisibleChanged: if (visible) forceActiveFocus()
                onAccepted: {
                    if (greeter && text.length > 0) {
                        greeter.submit(text);
                        text = "";
                    }
                }

                Keys.onLeftPressed: event => {
                    greeter.usersExpanded = true;
                    event.accepted = true;
                }

                Keys.onUpPressed: event => {
                    greeter.usersExpanded = true;
                    event.accepted = true;
                }

                Keys.onDownPressed: event => {
                    greeter.sessionsExpanded = true;
                    event.accepted = true;
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.preferredHeight: 18
                spacing: 8
                visible: screen.hasUser && greeter.stage !== "finished" && !screen.choosingUser

                Text {
                    Layout.fillWidth: true
                    visible: screen.enteringPassword
                    color: screen.busy ? screen.cAccent : screen.cSubtext
                    font.family: screen.cFont
                    font.pixelSize: screen.cFontTiny
                    text: screen.busy ? screen.busyLabel : "Enter to sign in"
                }

                Item {
                    Layout.preferredWidth: 20
                    Layout.preferredHeight: 18
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                    Glyph {
                        anchors.centerIn: parent
                        theme: screen.theme
                        name: "gear"
                        size: screen.cFontSmall
                        color: sessionArea.containsMouse || screen.choosingSession ? screen.cAccent : screen.cSubtext
                    }

                    MouseArea {
                        id: sessionArea
                        anchors.fill: parent
                        anchors.margins: -8
                        hoverEnabled: true
                        onClicked: greeter.sessionsExpanded = !greeter.sessionsExpanded
                    }

                    ToolTip.visible: sessionArea.containsMouse && !screen.choosingSession
                    ToolTip.text: greeter && greeter.chosenSession ? greeter.chosenSession.name : "No session"
                }
            }

            ListView {
                id: sessionList

                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(4, Math.max(1, greeter ? greeter.sessions.length : 1)) * 32
                visible: screen.choosingSession
                clip: true
                focus: visible
                model: greeter ? greeter.sessions : []
                currentIndex: greeter ? greeter.sessionCursor : -1

                onVisibleChanged: if (visible) forceActiveFocus()

                delegate: Rectangle {
                    required property var modelData
                    required property int index

                    width: ListView.view.width
                    height: 32
                    radius: 6
                    color: index === (greeter ? greeter.sessionCursor : -1) ? screen.cSurfaceAlt : "transparent"

                    Text {
                        anchors {
                            left: parent.left
                            leftMargin: 12
                            verticalCenter: parent.verticalCenter
                        }

                        color: screen.cSubtext
                        font.family: screen.cFont
                        font.pixelSize: screen.cFontTiny
                        text: modelData.name
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: greeter.chooseSession(modelData.id)
                    }
                }

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Down) {
                        greeter.sessionCursor = Math.min(greeter.sessions.length - 1, greeter.sessionCursor + 1);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Up) {
                        greeter.sessionCursor = Math.max(0, greeter.sessionCursor - 1);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        if (greeter.sessions.length > 0)
                            greeter.chooseSession(greeter.sessions[greeter.sessionCursor].id);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Escape) {
                        greeter.sessionsExpanded = false;
                        event.accepted = true;
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                visible: screen.enteringPrompt && greeter.promptText.length > 0
                color: screen.cSubtext
                font.family: screen.cFont
                font.pixelSize: screen.cFontSmall
                text: greeter ? greeter.promptText : ""
            }

            TextField {
                id: promptField

                Layout.fillWidth: true
                Layout.preferredHeight: 40
                visible: screen.enteringPrompt
                enabled: !screen.busy
                opacity: screen.busy ? 0.5 : 1.0
                color: screen.cText
                font.family: screen.cFont
                font.pixelSize: screen.cFontSmall
                echoMode: greeter && greeter.promptKind === "secret" ? TextInput.Password : TextInput.Normal
                placeholderText: "Response"
                placeholderTextColor: screen.cOverlay
                leftPadding: 12
                rightPadding: 12
                background: Rectangle {
                    radius: 8
                    color: screen.cSurfaceAlt
                    border.width: 1
                    border.color: promptField.activeFocus ? screen.cAccent : screen.cBorder
                }
                onVisibleChanged: if (visible) forceActiveFocus()
                onAccepted: {
                    if (greeter && text.length > 0) {
                        greeter.respond(text);
                        text = "";
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                visible: greeter !== null && greeter.stage === "finished"
                spacing: 4

                Text {
                    Layout.fillWidth: true
                    color: screen.cSubtext
                    font.family: screen.cFont
                    font.pixelSize: screen.cFontTiny
                    text: greeter ? "User · " + greeter.chosenUser : ""
                }

                Text {
                    Layout.fillWidth: true
                    color: screen.cSubtext
                    font.family: screen.cFont
                    font.pixelSize: screen.cFontTiny
                    text: greeter && greeter.chosenSession ? "Session · " + greeter.chosenSession.name : ""
                }

                Text {
                    Layout.fillWidth: true
                    color: screen.cOverlay
                    font.family: screen.cFont
                    font.pixelSize: screen.cFontTiny
                    text: greeter && greeter.mock ? "Would start now (mock)." : "Starting…"
                }
            }

            Text {
                Layout.fillWidth: true
                visible: greeter !== null && greeter.message.length > 0
                color: screen.cSubtext
                font.family: screen.cFont
                font.pixelSize: screen.cFontTiny
                wrapMode: Text.WordWrap
                text: greeter ? greeter.message : ""
            }

            Text {
                Layout.fillWidth: true
                visible: greeter !== null && greeter.error.length > 0
                color: screen.cDanger
                font.family: screen.cFont
                font.pixelSize: screen.cFontTiny
                wrapMode: Text.WordWrap
                text: greeter ? greeter.error : ""
            }
        }
    }

    Keys.onPressed: event => {
        if (!greeter)
            return;
        if (event.key === Qt.Key_Q && event.modifiers & Qt.ControlModifier) {
            greeter.quit();
            event.accepted = true;
        }
    }
}
