import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell.Io

FocusScope {
    id: screen

    property var greeter: null

    readonly property var theme: greeter ? greeter.theme : null
    readonly property color cBase: theme ? theme.base : "#000000"
    readonly property color cMantle: theme ? theme.mantle : "#181825"
    readonly property color cSurface0: theme ? theme.surface0 : "#313244"
    readonly property color cSurface1: theme ? theme.surface1 : "#45475a"
    readonly property color cOverlay: theme ? theme.overlay0 : "#6c7086"
    readonly property color cSubtext: theme ? theme.subtext1 : "#bac2de"
    readonly property color cText: theme ? theme.text : "#cdd6f4"
    readonly property color cAccent: theme ? theme.blue : "#89b4fa"
    readonly property color cRed: theme ? theme.red : "#f38ba8"
    readonly property int cRadius: theme ? theme.radius : 16
    readonly property string cFont: theme ? theme.fontFamily : "sans-serif"

    readonly property bool hasUser: greeter !== null && greeter.chosenUser.length > 0
    readonly property bool choosingUser: greeter !== null && greeter.usersExpanded
    readonly property bool choosingSession: greeter !== null && greeter.sessionsExpanded
    readonly property bool enteringPassword: greeter !== null && greeter.stage === "login" && hasUser && !choosingUser && !choosingSession
    readonly property bool enteringPrompt: greeter !== null && greeter.stage === "prompt"
    readonly property bool busy: greeter !== null && greeter.busy

    property bool powerMenuOpen: false
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

    Rectangle {
        anchors.fill: parent
        color: screen.cBase
    }

    Item {
        id: topBar

        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
        }

        height: 40

        Text {
            anchors {
                left: parent.left
                leftMargin: 20
                verticalCenter: parent.verticalCenter
            }

            color: screen.cSubtext
            font.family: screen.cFont
            font.pixelSize: 13
            text: hostnameFile.text().trim()
        }

        Text {
            id: clock

            anchors {
                horizontalCenter: parent.horizontalCenter
                verticalCenter: parent.verticalCenter
            }

            color: screen.cSubtext
            font.family: screen.cFont
            font.pixelSize: 13
            font.bold: true
            text: Qt.formatDateTime(new Date(), "HH:mm")

            Timer {
                interval: 1000
                running: true
                repeat: true
                onTriggered: clock.text = Qt.formatDateTime(new Date(), "HH:mm")
            }
        }
    }

    FileView {
        id: hostnameFile
        path: "/etc/hostname"
        blockLoading: true
    }

    Rectangle {
        id: card

        anchors.centerIn: parent
        width: 360
        height: layout.implicitHeight + 40
        radius: screen.cRadius
        color: screen.cSurface0
        border.width: 1
        border.color: screen.cSurface1

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
                    font.pixelSize: 18

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
                    font.pixelSize: 20
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
                    color: index === (greeter ? greeter.userCursor : -1) ? screen.cSurface1 : "transparent"

                    Text {
                        anchors {
                            left: parent.left
                            leftMargin: 12
                            verticalCenter: parent.verticalCenter
                        }

                        color: screen.cText
                        font.family: screen.cFont
                        font.pixelSize: 15
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
                echoMode: TextInput.Password
                placeholderText: "Password"
                leftPadding: 12
                rightPadding: 12
                background: Rectangle {
                    radius: 8
                    color: screen.cMantle
                    border.width: 1
                    border.color: passwordField.activeFocus ? screen.cAccent : screen.cSurface1
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
                    color: screen.busy ? screen.cAccent : screen.cOverlay
                    font.family: screen.cFont
                    font.pixelSize: 11
                    text: screen.busy ? screen.busyLabel : "Enter to sign in"
                }

                Item {
                    Layout.preferredWidth: 20
                    Layout.preferredHeight: 18
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                    Text {
                        anchors.centerIn: parent
                        color: sessionArea.containsMouse || screen.choosingSession ? screen.cAccent : screen.cSubtext
                        font.family: "JetBrainsMono Nerd Font"
                        font.pixelSize: 15
                        text: "\uf013"
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
                    color: index === (greeter ? greeter.sessionCursor : -1) ? screen.cSurface1 : "transparent"

                    Text {
                        anchors {
                            left: parent.left
                            leftMargin: 12
                            verticalCenter: parent.verticalCenter
                        }

                        color: screen.cSubtext
                        font.family: screen.cFont
                        font.pixelSize: 13
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
                font.pixelSize: 14
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
                echoMode: greeter && greeter.promptKind === "secret" ? TextInput.Password : TextInput.Normal
                placeholderText: "Response"
                leftPadding: 12
                rightPadding: 12
                background: Rectangle {
                    radius: 8
                    color: screen.cMantle
                    border.width: 1
                    border.color: promptField.activeFocus ? screen.cAccent : screen.cSurface1
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
                    font.pixelSize: 13
                    text: greeter ? "User · " + greeter.chosenUser : ""
                }

                Text {
                    Layout.fillWidth: true
                    color: screen.cSubtext
                    font.family: screen.cFont
                    font.pixelSize: 13
                    text: greeter && greeter.chosenSession ? "Session · " + greeter.chosenSession.name : ""
                }

                Text {
                    Layout.fillWidth: true
                    color: screen.cOverlay
                    font.family: screen.cFont
                    font.pixelSize: 11
                    text: greeter && greeter.mock ? "Would start now (mock)." : "Starting…"
                }
            }

            Text {
                Layout.fillWidth: true
                visible: greeter !== null && greeter.message.length > 0
                color: screen.cSubtext
                font.family: screen.cFont
                font.pixelSize: 13
                wrapMode: Text.WordWrap
                text: greeter ? greeter.message : ""
            }

            Text {
                Layout.fillWidth: true
                visible: greeter !== null && greeter.error.length > 0
                color: screen.cRed
                font.family: screen.cFont
                font.pixelSize: 13
                wrapMode: Text.WordWrap
                text: greeter ? greeter.error : ""
            }
        }
    }

    Item {
        id: powerControl

        anchors {
            right: parent.right
            bottom: parent.bottom
            margins: 20
        }

        width: 40
        height: 40

        Column {
            anchors {
                bottom: parent.top
                bottomMargin: 8
                right: parent.right
            }

            spacing: 6
            visible: screen.powerMenuOpen

            Rectangle {
                width: 120
                height: 34
                radius: 8
                color: restartArea.containsMouse ? screen.cSurface1 : screen.cMantle
                border.width: 1
                border.color: screen.cSurface1

                Text {
                    anchors.centerIn: parent
                    text: "Restart"
                    color: screen.cText
                    font.family: screen.cFont
                    font.pixelSize: 13
                }

                MouseArea {
                    id: restartArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        screen.powerMenuOpen = false;
                        greeter.power("reboot");
                    }
                }
            }

            Rectangle {
                width: 120
                height: 34
                radius: 8
                color: offArea.containsMouse ? screen.cSurface1 : screen.cMantle
                border.width: 1
                border.color: screen.cSurface1

                Text {
                    anchors.centerIn: parent
                    text: "Shut down"
                    color: screen.cRed
                    font.family: screen.cFont
                    font.pixelSize: 13
                }

                MouseArea {
                    id: offArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        screen.powerMenuOpen = false;
                        greeter.power("off");
                    }
                }
            }
        }

        Rectangle {
            anchors.fill: parent
            radius: 20
            color: powerArea.containsMouse || screen.powerMenuOpen ? screen.cSurface1 : screen.cMantle
            border.width: 1
            border.color: screen.cSurface1

            Text {
                anchors.centerIn: parent
                text: "⏻"
                color: powerArea.containsMouse ? screen.cRed : screen.cSubtext
                font.pixelSize: 18
            }

            MouseArea {
                id: powerArea
                anchors.fill: parent
                hoverEnabled: true
                onClicked: screen.powerMenuOpen = !screen.powerMenuOpen
            }
        }
    }

    Keys.onPressed: event => {
        if (!greeter)
            return;
        if (event.key === Qt.Key_Q && event.modifiers & Qt.ControlModifier) {
            greeter.quit();
            event.accepted = true;
        } else if (event.key === Qt.Key_Escape && screen.powerMenuOpen) {
            screen.powerMenuOpen = false;
            event.accepted = true;
        }
    }
}
