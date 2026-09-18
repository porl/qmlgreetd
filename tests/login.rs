use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use qmlgreetd::mock::{self, Scenario};
use qmlgreetd::protocol::{AuthMessageKind, Event};

const TIMEOUT: Duration = Duration::from_secs(10);

struct Harness {
    socket: PathBuf,
    dir: PathBuf,
    shutdown: Arc<AtomicBool>,
    mock: Option<thread::JoinHandle<()>>,
}

impl Harness {
    fn start(scenario: Scenario) -> Self {
        let dir = unique_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("greetd.sock");
        let shutdown = Arc::new(AtomicBool::new(false));
        let mock = {
            let socket = socket.clone();
            let shutdown = shutdown.clone();
            thread::spawn(move || mock::serve(&socket, scenario, shutdown).unwrap())
        };

        let deadline = Instant::now() + TIMEOUT;
        while !socket.exists() {
            assert!(Instant::now() < deadline, "the mock socket never appeared");
            thread::sleep(Duration::from_millis(5));
        }

        Self {
            socket,
            dir,
            shutdown,
            mock: Some(mock),
        }
    }

    fn transport(&self) -> Transport {
        let mut child = Command::new(env!("CARGO_BIN_EXE_qmlgreetd"))
            .arg("transport")
            .env("GREETD_SOCK", &self.socket)
            .env("QMLGREETD_TIMEOUT", "2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let (sender, receiver) = mpsc::channel();
        let stdout_handle = thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        let stderr_handle = thread::spawn(move || {
            let mut buffer = String::new();
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                buffer.push_str(&line);
                buffer.push('\n');
            }
            buffer
        });

        Transport {
            child,
            stdin,
            receiver,
            stdout_handle,
            stderr_handle,
            transcript: Vec::new(),
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.mock.take() {
            let _ = handle.join();
        }
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

struct Transport {
    child: Child,
    stdin: ChildStdin,
    receiver: Receiver<String>,
    stdout_handle: thread::JoinHandle<()>,
    stderr_handle: thread::JoinHandle<String>,
    transcript: Vec<String>,
}

impl Transport {
    fn send(&mut self, command: &str) {
        writeln!(self.stdin, "{command}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn create_session(&mut self, username: &str) {
        self.send(&format!(
            r#"{{"cmd":"create_session","username":"{username}"}}"#
        ));
    }

    fn respond(&mut self, value: &str) {
        self.send(&format!(r#"{{"cmd":"respond","value":"{value}"}}"#));
    }

    fn event(&mut self) -> Event {
        let line = self
            .receiver
            .recv_timeout(TIMEOUT)
            .expect("expected an event");
        self.transcript.push(line.clone());
        serde_json::from_str(&line).unwrap()
    }

    fn finish(mut self) -> Outcome {
        drop(self.stdin);
        let status = self.child.wait().unwrap();
        let _ = self.stdout_handle.join();
        let stderr = self.stderr_handle.join().unwrap();
        Outcome {
            success: status.success(),
            stderr,
            transcript: self.transcript,
        }
    }
}

struct Outcome {
    success: bool,
    stderr: String,
    transcript: Vec<String>,
}

#[test]
fn a_wrong_password_retries_then_succeeds() {
    let harness = Harness::start(Scenario::Normal {
        password: "hunter2".into(),
    });
    let mut transport = harness.transport();

    assert_eq!(transport.event(), Event::Connected);

    transport.create_session("alice");
    assert!(matches!(
        transport.event(),
        Event::AuthMessage {
            kind: AuthMessageKind::Secret,
            ..
        }
    ));

    transport.respond("wrong");
    assert_eq!(
        transport.event(),
        Event::AuthError {
            text: "Incorrect password. Try again.".into()
        }
    );

    transport.create_session("alice");
    assert!(matches!(
        transport.event(),
        Event::AuthMessage {
            kind: AuthMessageKind::Secret,
            ..
        }
    ));

    transport.respond("hunter2");
    assert_eq!(transport.event(), Event::Authenticated);

    transport.send(r#"{"cmd":"start_session","exec":["/bin/true"],"env":[]}"#);
    assert_eq!(transport.event(), Event::Started);

    let outcome = transport.finish();
    assert!(outcome.success, "the transport exited with an error");
    assert_password_never_leaks(&outcome, &["hunter2", "wrong"]);
}

#[test]
fn an_informational_message_is_acknowledged_automatically() {
    let harness = Harness::start(Scenario::Info {
        password: "hunter2".into(),
    });
    let mut transport = harness.transport();

    assert_eq!(transport.event(), Event::Connected);
    transport.create_session("alice");
    assert_eq!(
        transport.event(),
        Event::AuthMessage {
            kind: AuthMessageKind::Info,
            text: "Welcome".into()
        }
    );
    assert_eq!(
        transport.event(),
        Event::AuthMessage {
            kind: AuthMessageKind::Secret,
            text: "Password: ".into()
        }
    );
    transport.respond("hunter2");
    assert_eq!(transport.event(), Event::Authenticated);

    let outcome = transport.finish();
    assert!(outcome.success);
    assert_password_never_leaks(&outcome, &["hunter2"]);
}

#[test]
fn a_visible_prompt_is_passed_through_before_the_secret_one() {
    let harness = Harness::start(Scenario::Visible {
        question: "Login: ".into(),
        password: "hunter2".into(),
    });
    let mut transport = harness.transport();

    assert_eq!(transport.event(), Event::Connected);
    transport.create_session("alice");
    assert_eq!(
        transport.event(),
        Event::AuthMessage {
            kind: AuthMessageKind::Visible,
            text: "Login: ".into()
        }
    );
    transport.respond("alice");
    assert_eq!(
        transport.event(),
        Event::AuthMessage {
            kind: AuthMessageKind::Secret,
            text: "Password: ".into()
        }
    );
    transport.respond("hunter2");
    assert_eq!(transport.event(), Event::Authenticated);

    let outcome = transport.finish();
    assert!(outcome.success);
}

#[test]
fn a_hung_conversation_times_out_and_exits() {
    let harness = Harness::start(Scenario::NoReply);
    let mut transport = harness.transport();

    assert_eq!(transport.event(), Event::Connected);
    transport.create_session("alice");

    let event = transport.event();
    assert!(
        matches!(event, Event::Exited { .. }),
        "expected the watchdog to end the conversation, got {event:?}"
    );

    let outcome = transport.finish();
    assert!(!outcome.success, "a timed-out transport must exit non-zero");
}

#[test]
fn the_mock_refuses_to_run_under_greetd() {
    let output = Command::new(env!("CARGO_BIN_EXE_qmlgreetd"))
        .arg("mock-greetd")
        .env("GREETD_SOCK", "/tmp/not-a-real-greetd.sock")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing"));
}

#[test]
fn enumerate_prints_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_qmlgreetd"))
        .arg("enumerate")
        .output()
        .unwrap();

    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["users"].is_array());
    assert!(report["sessions"].is_array());
}

fn assert_password_never_leaks(outcome: &Outcome, secrets: &[&str]) {
    for secret in secrets {
        assert!(
            !outcome.stderr.contains(secret),
            "a password appeared on stderr"
        );
        for line in &outcome.transcript {
            assert!(
                !line.contains(secret),
                "a password appeared in the UI event stream"
            );
        }
    }
}

fn unique_dir() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("qmlgreetd-{}-{}", std::process::id(), stamp))
}
