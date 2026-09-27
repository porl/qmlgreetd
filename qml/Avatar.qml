import QtQuick
import Quickshell.Widgets

// A user's avatar: the AccountsService icon when there is one, else the display
// name's initial on a muted disc. "No icon" is the normal case: AccountsService
// reports $HOME/.face for every user whether or not it exists.
ClippingRectangle {
    id: avatar

    property string source: ""
    property string name: ""
    property color textColor: "#cdd6f4"
    property string fontFamily: "sans-serif"

    radius: width / 2
    color: "#45475a"

    Text {
        anchors.centerIn: parent
        visible: avatarImage.status !== Image.Ready
        text: avatar.initial()
        color: avatar.textColor
        font.family: avatar.fontFamily
        font.pixelSize: Math.round(avatar.height * 0.5)
    }

    Image {
        id: avatarImage

        anchors.fill: parent
        source: avatar.source
        fillMode: Image.PreserveAspectCrop
        sourceSize {
            width: 96
            height: 96
        }
        asynchronous: true
    }

    function initial() {
        var trimmed = avatar.name.trim();
        return trimmed.length > 0 ? trimmed.charAt(0).toUpperCase() : "";
    }
}
