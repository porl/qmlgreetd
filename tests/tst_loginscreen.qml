import QtQuick
import QtTest
import "../qml"

// The bundled login card's keyboard behaviour (qml/LoginScreen.qml). The card
// ships inside Quickshell, whose QML modules only load in-process, so the test
// tree merges qcommon in (as the package does) and stubs the two Quickshell
// module leaves the card touches.
TestCase {
    name: "LoginScreen"

    // Input tests need the window exposed before they run.
    when: windowShown
    visible: true
    width: 600
    height: 600

    Theme { id: testTheme }

    QtObject {
        id: greeterStub

        property string stage: "login"
        property string chosenUser: "porl"
        property var chosenUserObject: ({ display_name: "Porl", username: "porl", avatar: "" })
        property var users: [({ display_name: "Porl", username: "porl", avatar: "" })]
        property bool usersExpanded: false
        property bool sessionsExpanded: false
        property var sessions: [({ id: "hyprland", name: "Hyprland" })]
        property int userCursor: 0
        property int sessionCursor: 0
        property var chosenSession: ({ id: "hyprland", name: "Hyprland" })
        property bool busy: false
        property string message: ""
        property string error: ""
        property string promptText: ""
        property string promptKind: "secret"
        property bool mock: true
        property var theme: testTheme

        property var submitted: []

        function chooseUser(name) {}
        function chooseSession(id) {}
        function submit(password) { submitted.push(password); }
        function respond(text) {}
        function quit() {}
    }

    LoginScreen {
        id: screen

        anchors.fill: parent
        greeter: greeterStub
    }

    function passwordField() {
        var field = findChild(screen, "passwordField");
        verify(field !== null, "found the password field");
        verify(field.visible, "the password field is showing");
        field.forceActiveFocus();
        verify(field.activeFocus, "the password field has active focus");
        return field;
    }

    function cleanup() {
        greeterStub.stage = "login";
        greeterStub.usersExpanded = false;
        greeterStub.sessionsExpanded = false;
        greeterStub.promptText = "";
        greeterStub.submitted = [];
        greeterStub.message = "";
        greeterStub.error = "";
    }

    function test_escape_clears_a_typed_password() {
        greeterStub.submitted = [];
        var field = passwordField();
        field.text = "hunter2";
        keyClick(Qt.Key_Escape);
        compare(field.text, "", "escape clears the field");
        compare(greeterStub.submitted.length, 0, "escape does not submit");
    }

    function test_escape_from_the_user_list_clears_and_returns_to_the_field() {
        var field = passwordField();
        field.text = "hunter2";
        greeterStub.usersExpanded = true;
        var list = findChild(screen, "userList");
        verify(list !== null, "found the user list");
        list.forceActiveFocus();
        keyClick(Qt.Key_Escape);
        compare(greeterStub.usersExpanded, false, "escape closes the user list");
        compare(field.text, "", "escape clears the typed password too");
        verify(field.activeFocus, "focus is back on the password field");
    }

    function test_escape_from_the_session_list_clears_and_returns_to_the_field() {
        var field = passwordField();
        field.text = "hunter2";
        greeterStub.sessionsExpanded = true;
        var list = findChild(screen, "sessionList");
        verify(list !== null, "found the session list");
        list.forceActiveFocus();
        keyClick(Qt.Key_Escape);
        compare(greeterStub.sessionsExpanded, false, "escape closes the session list");
        compare(field.text, "", "escape clears the typed password too");
        verify(field.activeFocus, "focus is back on the password field");
    }

    function test_escape_clears_a_prompt_response() {
        greeterStub.stage = "prompt";
        greeterStub.promptText = "One-time code";
        var prompt = findChild(screen, "promptField");
        verify(prompt !== null, "found the prompt field");
        verify(prompt.visible, "the prompt field is showing");
        prompt.forceActiveFocus();
        verify(prompt.activeFocus, "the prompt field has active focus");
        prompt.text = "123456";
        keyClick(Qt.Key_Escape);
        compare(prompt.text, "", "escape clears the prompt response");
        verify(prompt.activeFocus, "focus stays on the prompt field");
    }

    function test_escape_on_an_empty_field_is_harmless() {
        var field = passwordField();
        field.text = "";
        keyClick(Qt.Key_Escape);
        compare(field.text, "");
    }

    function test_enter_still_submits_and_clears() {
        greeterStub.submitted = [];
        var field = passwordField();
        field.text = "hunter2";
        keyClick(Qt.Key_Return);
        compare(greeterStub.submitted.length, 1, "enter submits once");
        compare(greeterStub.submitted[0], "hunter2");
        compare(field.text, "");
    }
}
