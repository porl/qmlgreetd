import QtQuick
import Quickshell
import Quickshell.Io

QtObject {
    id: greeter

    property Theme theme: Theme {}

    readonly property string binary: Quickshell.env("QMLGREETD_BIN") || "qmlgreetd"
    readonly property bool mock: Quickshell.env("QMLGREETD_MOCK") === "1"
    readonly property url uiPath: {
        var custom = Quickshell.env("QMLGREETD_UI");
        if (custom)
            return Qt.resolvedUrl(custom);
        return Qt.resolvedUrl("LoginScreen.qml");
    }

    // Optional background image; empty means no wallpaper.
    readonly property string wallpaper: Quickshell.env("QMLGREETD_WALLPAPER") || ""

    property bool connected: false
    property bool busy: false
    property string stage: "login"
    property string message: ""
    property string error: ""

    property var users: []
    property var sessions: []
    property string rememberedUser: ""
    property var sessionByUser: ({})

    property string chosenUser: ""
    property string chosenSessionId: ""
    property bool usersExpanded: false
    property bool sessionsExpanded: false
    property int userCursor: 0
    property int sessionCursor: 0

    property string promptKind: "secret"
    property string promptText: ""
    property string pendingPassword: ""

    readonly property var chosenUserObject: findUser(chosenUser)
    readonly property var chosenSession: findSession(chosenSessionId)

    function findUser(name) {
        for (var i = 0; i < users.length; i++) {
            if (users[i].username === name)
                return users[i];
        }
        return null;
    }

    function findSession(id) {
        for (var i = 0; i < sessions.length; i++) {
            if (sessions[i].id === id)
                return sessions[i];
        }
        return null;
    }

    function sessionDefaultFor(user) {
        var remembered = sessionByUser ? sessionByUser[user] : undefined;
        if (remembered && findSession(remembered))
            return remembered;
        var fallback = rememberedUser && sessionByUser ? sessionByUser[rememberedUser] : undefined;
        if (fallback && findSession(fallback))
            return fallback;
        return sessions.length > 0 ? sessions[0].id : "";
    }

    function applyReport(report) {
        users = report.users || [];
        sessions = report.sessions || [];
        var remembered = report.state || {};
        rememberedUser = remembered.last_user || "";
        sessionByUser = remembered.session_by_user || {};

        var initial = "";
        if (rememberedUser && findUser(rememberedUser))
            initial = rememberedUser;
        else if (users.length === 1)
            initial = users[0].username;

        if (initial.length > 0) {
            chosenUser = initial;
            chosenSessionId = sessionDefaultFor(initial);
        } else {
            chosenUser = "";
            usersExpanded = true;
        }
    }

    function chooseUser(name) {
        chosenUser = name;
        chosenSessionId = sessionDefaultFor(name);
        usersExpanded = false;
        error = "";
    }

    function chooseSession(id) {
        chosenSessionId = id;
        sessionsExpanded = false;
    }

    function submit(value) {
        if (chosenUser.length === 0 || value.length === 0)
            return;
        if (chosenSessionId.length === 0) {
            error = "No session is available to start.";
            return;
        }
        remember(chosenUser, chosenSessionId);
        pendingPassword = value;
        error = "";
        message = "";
        busy = true;
        send({ "cmd": "create_session", "username": chosenUser });
    }

    function respond(value) {
        busy = true;
        send({ "cmd": "respond", "value": value });
    }

    function startSession() {
        var session = chosenSession;
        if (session === null)
            return;
        send({ "cmd": "start_session", "exec": session.exec, "env": session.env });
    }

    function cancelSession() {
        send({ "cmd": "cancel_session" });
    }

    function send(object) {
        transport.write(JSON.stringify(object) + "\n");
    }

    function remember(user, session) {
        var args = [binary, "remember", "--user", user];
        if (session && session.length > 0)
            args.push("--session", session);
        Quickshell.execDetached(args);
    }

    function quit() {
        Quickshell.execDetached(["kill", "-TERM", "" + Quickshell.processId]);
    }

    readonly property string powerMode: Quickshell.env("QMLGREETD_POWER") || "disabled"

    function power(action) {
        if (mock) {
            if (action === "off")
                quit();
            else
                Quickshell.reload(true);
            return;
        }
        if (powerMode === "disabled") {
            error = "Power actions are disabled.";
            return;
        }
        Quickshell.execDetached([binary, "power", action]);
    }

    function handleEvent(line) {
        var event;
        try {
            event = JSON.parse(line);
        } catch (parseError) {
            return;
        }
        switch (event.event) {
        case "connected":
            connected = true;
            break;
        case "auth_message":
            if (event.kind === "secret" && pendingPassword.length > 0) {
                var secret = pendingPassword;
                pendingPassword = "";
                respond(secret);
            } else if (event.kind === "secret" || event.kind === "visible") {
                promptKind = event.kind;
                promptText = event.text;
                stage = "prompt";
                busy = false;
            } else {
                message = event.text;
            }
            break;
        case "auth_error":
            pendingPassword = "";
            error = event.text;
            stage = "login";
            busy = false;
            break;
        case "error":
            pendingPassword = "";
            error = event.text;
            busy = false;
            break;
        case "authenticated":
            stage = "login";
            sessionsExpanded = false;
            busy = true;
            startSession();
            break;
        case "started":
            stage = "finished";
            if (!mock)
                quit();
            break;
        case "cancelled":
            stage = "login";
            busy = false;
            break;
        case "exited":
            pendingPassword = "";
            error = event.reason;
            stage = "login";
            busy = false;
            if (!mock)
                quit();
            break;
        }
    }

    property Process transport: Process {
        command: [greeter.binary, "transport"]
        stdinEnabled: true
        running: true
        stdout: SplitParser {
            onRead: data => greeter.handleEvent(data)
        }
        stderr: SplitParser {
            onRead: data => console.warn("qmlgreetd transport: " + data)
        }
        onExited: {
            greeter.connected = false;
            greeter.busy = false;
            if (greeter.stage !== "finished" && greeter.error.length === 0)
                greeter.error = "The greetd connection ended.";
        }
    }

    property Process enumerator: Process {
        command: [greeter.binary, "enumerate"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                var report;
                try {
                    report = JSON.parse(this.text);
                } catch (parseError) {
                    greeter.error = "Could not load the user and session list.";
                    return;
                }
                greeter.applyReport(report);
            }
        }
    }
}
