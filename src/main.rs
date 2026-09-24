use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use qmlgreetd::enumerate::{self, Report};
use qmlgreetd::state::{self, State};
use qmlgreetd::{mock, power, transport};

const USAGE: &str = "\
qmlgreetd - a customizable Quickshell greeter for greetd

usage: qmlgreetd <command>

commands:
  transport     bridge greetd IPC to line-delimited JSON on stdio
  enumerate     list login users, sessions, and remembered selections
  remember      record the last user and their session
  power         shut down, reboot, suspend or hibernate (off|reboot|suspend|hibernate)
  mock-greetd   run the fake greetd backend (development only)";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("transport") => ExitCode::from(transport::run()),
        Some("enumerate") => enumerate_command(),
        Some("remember") => remember_command(),
        Some("power") => power_command(),
        Some("mock-greetd") => mock_command(),
        Some("-h") | Some("--help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("qmlgreetd: unknown command: {other}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        None => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn enumerate_command() -> ExitCode {
    let data_dirs = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    let report = Report {
        users: enumerate::users::list_users(
            &enumerate::users::read_passwd(),
            enumerate::users::uid_min(),
        ),
        sessions: enumerate::sessions::load_sessions(&enumerate::sessions::session_dirs(
            &data_dirs,
        )),
        state: State::load(&state::state_path()),
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("qmlgreetd: could not encode the enumeration report: {error}");
            ExitCode::from(1)
        }
    }
}

fn remember_command() -> ExitCode {
    let mut user = None;
    let mut session = None;
    let mut args = std::env::args().skip(2);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--user" => user = args.next(),
            "--session" => session = args.next(),
            other => {
                eprintln!("qmlgreetd: unknown option: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(user) = user else {
        eprintln!("qmlgreetd: remember needs --user");
        return ExitCode::from(2);
    };

    let path = state::state_path();
    let mut state = State::load(&path);
    state.remember(&user, session.as_deref());
    match state.save(&path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("qmlgreetd: could not save remembered state: {error}");
            ExitCode::from(1)
        }
    }
}

fn power_command() -> ExitCode {
    let Some(value) = std::env::args().nth(2) else {
        eprintln!("qmlgreetd: power needs an action (off|reboot)");
        return ExitCode::from(2);
    };
    let Some(action) = power::parse_action(&value) else {
        eprintln!("qmlgreetd: unknown power action: {value}");
        return ExitCode::from(2);
    };

    let mode = power::parse_mode(std::env::var("QMLGREETD_POWER").ok().as_deref());
    match power::run(action, mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("qmlgreetd: {message}");
            ExitCode::from(1)
        }
    }
}

struct MockOptions {
    socket: PathBuf,
    scenario: mock::Scenario,
}

fn mock_command() -> ExitCode {
    if std::env::var_os("GREETD_SOCK").is_some() {
        eprintln!("qmlgreetd: refusing to run the mock backend while GREETD_SOCK is set");
        return ExitCode::from(2);
    }

    let options = match parse_mock_options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("qmlgreetd: {message}");
            return ExitCode::from(2);
        }
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    match mock::serve(&options.socket, options.scenario, shutdown) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("qmlgreetd: the mock backend failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn parse_mock_options() -> Result<MockOptions, String> {
    let mut socket = PathBuf::from("/tmp/qmlgreetd-mock.sock");
    let mut name = String::from("normal");
    let mut question = String::from("Username: ");
    let password = std::env::var("QMLGREETD_MOCK_PASSWORD").unwrap_or_else(|_| "password".into());

    let mut args = std::env::args().skip(2);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => socket = PathBuf::from(args.next().ok_or("--socket needs a value")?),
            "--scenario" => name = args.next().ok_or("--scenario needs a value")?,
            "--question" => question = args.next().ok_or("--question needs a value")?,
            other => return Err(format!("unknown option: {other}")),
        }
    }

    let scenario = match name.as_str() {
        "normal" => mock::Scenario::Normal { password },
        "visible" => mock::Scenario::Visible { question, password },
        "info" => mock::Scenario::Info { password },
        "secret-reprompt" => mock::Scenario::SecretReprompt { password },
        "auth-error" => mock::Scenario::AuthError,
        "no-reply" => mock::Scenario::NoReply,
        other => return Err(format!("unknown scenario: {other}")),
    };

    Ok(MockOptions { socket, scenario })
}
