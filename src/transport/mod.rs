pub mod frame;

use std::env;
use std::io::{self, BufRead, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::auth::machine::{Machine, Output, Phase};
use crate::greetd::{Request, Response};
use crate::protocol::{Command, Event};
use frame::{read_frame, write_frame};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

enum Input {
    Command(Command),
    Response(Response),
    GreetdEnded(String),
    InvalidCommand(String),
    Watchdog,
    UiEnded,
}

pub fn run() -> u8 {
    let Some(socket) = env::var_os("GREETD_SOCK") else {
        eprintln!("qmlgreetd: GREETD_SOCK is not set; not running under greetd");
        return 1;
    };

    let stream = match UnixStream::connect(&socket) {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("qmlgreetd: cannot connect to greetd: {error}");
            return 1;
        }
    };
    let reader = match stream.try_clone() {
        Ok(reader) => reader,
        Err(error) => {
            eprintln!("qmlgreetd: cannot duplicate the greetd socket: {error}");
            return 1;
        }
    };

    let (sender, receiver) = mpsc::channel();
    let greetd_sender = sender.clone();
    thread::spawn(move || read_greetd(reader, greetd_sender));
    let stdin_sender = sender.clone();
    thread::spawn(move || read_stdin(stdin_sender));

    let timeout = timeout_secs();
    let activity = Arc::new(Mutex::new(Instant::now()));
    let armed = Arc::new(AtomicBool::new(false));
    if timeout > 0 {
        let watchdog_sender = sender.clone();
        let activity = activity.clone();
        let armed = armed.clone();
        thread::spawn(move || watch(timeout, activity, armed, watchdog_sender));
    }

    let mut machine = Machine::new();
    let mut greetd = stream;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    emit(&mut stdout, &Event::Connected);

    let mut exit_code = 0;
    for input in receiver {
        if let Ok(mut last) = activity.lock() {
            *last = Instant::now();
        }
        let outputs = match input {
            Input::Command(command) => machine.handle_command(command),
            Input::Response(response) => machine.handle_response(response),
            Input::GreetdEnded(reason) => machine.handle_transport_failure(&reason),
            Input::InvalidCommand(message) => vec![Output::ToUi(Event::Error { text: message })],
            Input::Watchdog => machine.handle_transport_failure("timed out waiting for greetd"),
            Input::UiEnded => break,
        };
        armed.store(is_busy(machine.phase()), Ordering::Relaxed);
        for output in outputs {
            match output {
                Output::ToGreetd(request) => {
                    if let Err(error) = send_request(&mut greetd, &request) {
                        emit(
                            &mut stdout,
                            &Event::Exited {
                                reason: format!("failed to send to greetd: {error}"),
                            },
                        );
                        return 1;
                    }
                }
                Output::ToUi(event) => {
                    if matches!(event, Event::Exited { .. }) {
                        exit_code = 1;
                    }
                    emit(&mut stdout, &event);
                }
                Output::Quit => return exit_code,
            }
        }
    }
    exit_code
}

fn is_busy(phase: Phase) -> bool {
    matches!(
        phase,
        Phase::Authenticating | Phase::AwaitingSession | Phase::Starting | Phase::Cancelling
    )
}

fn timeout_secs() -> u64 {
    env::var("QMLGREETD_TIMEOUT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

fn watch(
    timeout: u64,
    activity: Arc<Mutex<Instant>>,
    armed: Arc<AtomicBool>,
    sender: Sender<Input>,
) {
    let timeout = Duration::from_secs(timeout);
    loop {
        thread::sleep(Duration::from_millis(250));
        if !armed.load(Ordering::Relaxed) {
            continue;
        }
        let elapsed = activity
            .lock()
            .map(|last| last.elapsed())
            .unwrap_or_default();
        if elapsed >= timeout {
            let _ = sender.send(Input::Watchdog);
            return;
        }
    }
}

fn emit(stdout: &mut impl Write, event: &Event) {
    if let Ok(json) = serde_json::to_string(event) {
        let _ = writeln!(stdout, "{json}");
        let _ = stdout.flush();
    }
}

fn send_request(stream: &mut UnixStream, request: &Request) -> io::Result<()> {
    let payload = serde_json::to_vec(request).map_err(to_io)?;
    write_frame(stream, &payload).map_err(to_io)
}

fn read_greetd(mut stream: UnixStream, sender: Sender<Input>) {
    loop {
        match read_frame(&mut stream) {
            Ok(Some(frame)) => match serde_json::from_slice::<Response>(&frame) {
                Ok(response) => {
                    if sender.send(Input::Response(response)).is_err() {
                        return;
                    }
                }
                Err(_) => {
                    let _ = sender.send(Input::GreetdEnded(
                        "greetd sent an unreadable message".to_string(),
                    ));
                    return;
                }
            },
            Ok(None) => {
                let _ = sender.send(Input::GreetdEnded(
                    "greetd closed the connection".to_string(),
                ));
                return;
            }
            Err(error) => {
                let _ = sender.send(Input::GreetdEnded(format!(
                    "greetd transport error: {error}"
                )));
                return;
            }
        }
    }
}

fn read_stdin(sender: Sender<Input>) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Command>(&line) {
            Ok(command) => {
                if sender.send(Input::Command(command)).is_err() {
                    return;
                }
            }
            Err(_) => {
                let _ = sender.send(Input::InvalidCommand(
                    "could not parse a command from the UI".to_string(),
                ));
            }
        }
    }
    let _ = sender.send(Input::UiEnded);
}

fn to_io(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}
