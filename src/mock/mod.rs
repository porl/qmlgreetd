use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::greetd::{AuthMessageType, ErrorType, Request, Response};
use crate::transport::frame::{read_frame, write_frame};

#[derive(Debug, Clone)]
pub enum Scenario {
    Normal { password: String },
    Visible { question: String, password: String },
    Info { password: String },
    SecretReprompt { password: String },
    AuthError,
    NoReply,
}

struct Conversation {
    scenario: Scenario,
    step: u32,
}

impl Conversation {
    fn new(scenario: Scenario) -> Self {
        Self { scenario, step: 0 }
    }

    fn handle(&mut self, request: &Request) -> Vec<Response> {
        match &self.scenario {
            Scenario::NoReply => Vec::new(),
            Scenario::Normal { password } => match request {
                Request::CreateSession { .. } => vec![secret("Password: ")],
                Request::PostAuthMessageResponse { response } => {
                    vec![check(password, response.as_deref())]
                }
                Request::StartSession { .. } | Request::CancelSession => vec![Response::Success],
            },
            Scenario::Visible { question, password } => match request {
                Request::CreateSession { .. } => {
                    self.step = 1;
                    vec![message(AuthMessageType::Visible, question)]
                }
                Request::PostAuthMessageResponse { .. } if self.step == 1 => {
                    self.step = 2;
                    vec![secret("Password: ")]
                }
                Request::PostAuthMessageResponse { response } if self.step == 2 => {
                    vec![check(password, response.as_deref())]
                }
                Request::StartSession { .. } | Request::CancelSession => vec![Response::Success],
                _ => Vec::new(),
            },
            Scenario::Info { password } => match request {
                Request::CreateSession { .. } => {
                    self.step = 1;
                    vec![message(AuthMessageType::Info, "Welcome")]
                }
                Request::PostAuthMessageResponse { .. } if self.step == 1 => {
                    self.step = 2;
                    vec![secret("Password: ")]
                }
                Request::PostAuthMessageResponse { response } if self.step == 2 => {
                    vec![check(password, response.as_deref())]
                }
                Request::StartSession { .. } | Request::CancelSession => vec![Response::Success],
                _ => Vec::new(),
            },
            Scenario::SecretReprompt { password } => match request {
                Request::CreateSession { .. } => {
                    self.step = 1;
                    vec![secret("Password: ")]
                }
                Request::PostAuthMessageResponse { .. } if self.step == 1 => {
                    self.step = 2;
                    vec![secret("Password expired. New password: ")]
                }
                Request::PostAuthMessageResponse { response } if self.step == 2 => {
                    vec![check(password, response.as_deref())]
                }
                Request::StartSession { .. } | Request::CancelSession => vec![Response::Success],
                _ => Vec::new(),
            },
            Scenario::AuthError => match request {
                Request::CreateSession { .. } => vec![secret("Password: ")],
                Request::PostAuthMessageResponse { .. } => vec![Response::Error {
                    error_type: ErrorType::AuthError,
                    description: "Authentication failure".to_string(),
                }],
                Request::StartSession { .. } | Request::CancelSession => vec![Response::Success],
            },
        }
    }
}

fn secret(text: &str) -> Response {
    message(AuthMessageType::Secret, text)
}

fn message(auth_message_type: AuthMessageType, text: &str) -> Response {
    Response::AuthMessage {
        auth_message_type,
        auth_message: text.to_string(),
    }
}

fn check(password: &str, response: Option<&str>) -> Response {
    if response == Some(password) {
        Response::Success
    } else {
        Response::Error {
            error_type: ErrorType::AuthError,
            description: "Sorry, try again.".to_string(),
        }
    }
}

pub fn serve(socket: &Path, scenario: Scenario, shutdown: Arc<AtomicBool>) -> io::Result<()> {
    let _ = std::fs::remove_file(socket);
    let listener = UnixListener::bind(socket)?;
    listener.set_nonblocking(true)?;

    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                handle_connection(stream, scenario.clone())?;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: UnixStream, scenario: Scenario) -> io::Result<()> {
    let mut conversation = Conversation::new(scenario);
    loop {
        let Some(frame) = read_frame(&mut stream).map_err(to_io)? else {
            return Ok(());
        };
        let Ok(request) = serde_json::from_slice::<Request>(&frame) else {
            return Ok(());
        };
        for response in conversation.handle(&request) {
            let payload = serde_json::to_vec(&response).map_err(to_io)?;
            write_frame(&mut stream, &payload).map_err(to_io)?;
        }
    }
}

fn to_io(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}
