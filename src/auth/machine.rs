use crate::exec::to_greetd_cmd;
use crate::greetd::{AuthMessageType, ErrorType, Request, Response};
use crate::protocol::{AuthMessageKind, Command, Event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Authenticating,
    AwaitingSession,
    Starting,
    Cancelling,
    Started,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    ToGreetd(Request),
    ToUi(Event),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    phase: Phase,
    pending_cancels: u32,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            pending_cancels: 0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn handle_command(&mut self, command: Command) -> Vec<Output> {
        match command {
            Command::CreateSession { username } => {
                if self.phase != Phase::Idle {
                    return vec![ui_error("a login is already in progress")];
                }
                self.phase = Phase::Authenticating;
                vec![Output::ToGreetd(Request::create_session(username))]
            }
            Command::Respond { value } => {
                if self.phase != Phase::Authenticating {
                    return vec![ui_error(
                        "no authentication prompt is waiting for a response",
                    )];
                }
                vec![Output::ToGreetd(Request::response(value))]
            }
            Command::StartSession { exec, env } => {
                if self.phase != Phase::AwaitingSession {
                    return vec![ui_error("not authenticated; cannot start a session")];
                }
                self.phase = Phase::Starting;
                vec![Output::ToGreetd(Request::StartSession {
                    cmd: to_greetd_cmd(&exec),
                    env,
                })]
            }
            Command::CancelSession => {
                if !matches!(self.phase, Phase::Authenticating | Phase::AwaitingSession) {
                    return vec![ui_error("no session is currently being configured")];
                }
                self.phase = Phase::Cancelling;
                vec![Output::ToGreetd(Request::CancelSession)]
            }
        }
    }

    pub fn handle_response(&mut self, response: Response) -> Vec<Output> {
        // greetd holds a session until cancel_session; consume its acknowledgement
        // so it is never mistaken for an auth success.
        if self.pending_cancels > 0 {
            self.pending_cancels -= 1;
            return Vec::new();
        }
        match self.phase {
            Phase::Authenticating => self.authenticating(response),
            Phase::AwaitingSession => match response {
                Response::Error {
                    error_type,
                    description,
                } => {
                    self.phase = Phase::Idle;
                    vec![Output::ToUi(error_event(error_type, description))]
                }
                _ => Vec::new(),
            },
            Phase::Starting => match response {
                Response::Success => {
                    self.phase = Phase::Started;
                    vec![Output::ToUi(Event::Started), Output::Quit]
                }
                Response::Error {
                    error_type,
                    description,
                } => {
                    self.phase = Phase::Idle;
                    vec![Output::ToUi(error_event(error_type, description))]
                }
                Response::AuthMessage { .. } => Vec::new(),
            },
            Phase::Cancelling => match response {
                Response::Success => {
                    self.phase = Phase::Idle;
                    vec![Output::ToUi(Event::Cancelled)]
                }
                Response::Error {
                    error_type,
                    description,
                } => {
                    self.phase = Phase::Idle;
                    vec![Output::ToUi(error_event(error_type, description))]
                }
                Response::AuthMessage { .. } => Vec::new(),
            },
            Phase::Idle | Phase::Started => Vec::new(),
        }
    }

    fn authenticating(&mut self, response: Response) -> Vec<Output> {
        match response {
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => {
                let mut outputs = vec![Output::ToUi(Event::AuthMessage {
                    kind: auth_message_kind(auth_message_type),
                    text: auth_message,
                })];
                if !auth_message_type.expects_response() {
                    outputs.push(Output::ToGreetd(Request::acknowledge()));
                }
                outputs
            }
            Response::Success => {
                self.phase = Phase::AwaitingSession;
                vec![Output::ToUi(Event::Authenticated)]
            }
            Response::Error {
                error_type,
                description,
            } => {
                // greetd holds the session until cancel_session.
                self.phase = Phase::Idle;
                self.pending_cancels += 1;
                vec![
                    Output::ToUi(error_event(error_type, description)),
                    Output::ToGreetd(Request::CancelSession),
                ]
            }
        }
    }

    pub fn handle_transport_failure(&mut self, reason: &str) -> Vec<Output> {
        self.phase = Phase::Idle;
        vec![
            Output::ToUi(Event::Exited {
                reason: reason.to_string(),
            }),
            Output::Quit,
        ]
    }
}

fn ui_error(text: &str) -> Output {
    Output::ToUi(Event::Error {
        text: text.to_string(),
    })
}

// greetd's description for a failed PAM conversation is a diagnostic string;
// specific reasons still arrive separately as auth_message events.
const AUTH_FAILURE_MESSAGE: &str = "Incorrect password. Try again.";

fn error_event(error_type: ErrorType, description: String) -> Event {
    match error_type {
        ErrorType::AuthError => Event::AuthError {
            text: AUTH_FAILURE_MESSAGE.to_string(),
        },
        ErrorType::Error => Event::Error { text: description },
    }
}

fn auth_message_kind(kind: AuthMessageType) -> AuthMessageKind {
    match kind {
        AuthMessageType::Secret => AuthMessageKind::Secret,
        AuthMessageType::Visible => AuthMessageKind::Visible,
        AuthMessageType::Info => AuthMessageKind::Info,
        AuthMessageType::Error => AuthMessageKind::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::AuthMessageKind;

    fn commands(machine: &mut Machine, command: Command) -> Vec<Output> {
        machine.handle_command(command)
    }

    fn response(machine: &mut Machine, response: Response) -> Vec<Output> {
        machine.handle_response(response)
    }

    fn auth_message(kind: AuthMessageType, text: &str) -> Response {
        Response::AuthMessage {
            auth_message_type: kind,
            auth_message: text.to_string(),
        }
    }

    fn create(machine: &mut Machine) {
        commands(
            machine,
            Command::CreateSession {
                username: "alice".into(),
            },
        );
    }

    #[test]
    fn secret_prompt_is_forwarded_and_awaits_a_response() {
        let mut machine = Machine::new();
        create(&mut machine);
        assert_eq!(machine.phase(), Phase::Authenticating);

        let outputs = response(
            &mut machine,
            auth_message(AuthMessageType::Secret, "Password: "),
        );
        assert_eq!(
            outputs,
            vec![Output::ToUi(Event::AuthMessage {
                kind: AuthMessageKind::Secret,
                text: "Password: ".into(),
            })]
        );
    }

    #[test]
    fn info_message_is_shown_and_acknowledged() {
        let mut machine = Machine::new();
        create(&mut machine);

        let outputs = response(&mut machine, auth_message(AuthMessageType::Info, "Welcome"));
        assert_eq!(
            outputs,
            vec![
                Output::ToUi(Event::AuthMessage {
                    kind: AuthMessageKind::Info,
                    text: "Welcome".into(),
                }),
                Output::ToGreetd(Request::acknowledge()),
            ]
        );
        assert_eq!(machine.phase(), Phase::Authenticating);
    }

    #[test]
    fn wrong_password_error_returns_to_idle_and_allows_a_retry() {
        let mut machine = Machine::new();
        create(&mut machine);
        let outputs = response(
            &mut machine,
            Response::Error {
                error_type: ErrorType::AuthError,
                description: "Sorry, try again.".into(),
            },
        );
        assert_eq!(
            outputs,
            vec![
                Output::ToUi(Event::AuthError {
                    text: "Incorrect password. Try again.".into(),
                }),
                Output::ToGreetd(Request::CancelSession),
            ]
        );
        assert_eq!(machine.phase(), Phase::Idle);

        assert!(response(&mut machine, Response::Success).is_empty());
        assert_eq!(machine.phase(), Phase::Idle);

        create(&mut machine);
        assert_eq!(machine.phase(), Phase::Authenticating);
    }

    #[test]
    fn a_stale_configured_session_is_released_on_error() {
        let mut machine = Machine::new();
        create(&mut machine);
        let outputs = response(
            &mut machine,
            Response::Error {
                error_type: ErrorType::Error,
                description: "a session is already being configured".into(),
            },
        );
        assert_eq!(
            outputs,
            vec![
                Output::ToUi(Event::Error {
                    text: "a session is already being configured".into(),
                }),
                Output::ToGreetd(Request::CancelSession),
            ]
        );

        assert!(response(&mut machine, Response::Success).is_empty());
        assert_eq!(machine.phase(), Phase::Idle);
        create(&mut machine);
        assert_eq!(machine.phase(), Phase::Authenticating);
    }

    #[test]
    fn a_cryptic_auth_error_is_simplified_for_the_ui() {
        let mut machine = Machine::new();
        create(&mut machine);
        let outputs = response(
            &mut machine,
            Response::Error {
                error_type: ErrorType::AuthError,
                description: "authentication error: pam_authenticate: AUTH_ERR".into(),
            },
        );
        assert_eq!(
            outputs,
            vec![
                Output::ToUi(Event::AuthError {
                    text: "Incorrect password. Try again.".into(),
                }),
                Output::ToGreetd(Request::CancelSession),
            ]
        );
    }

    #[test]
    fn success_after_authentication_moves_to_awaiting_session() {
        let mut machine = Machine::new();
        create(&mut machine);
        let outputs = response(&mut machine, Response::Success);
        assert_eq!(outputs, vec![Output::ToUi(Event::Authenticated)]);
        assert_eq!(machine.phase(), Phase::AwaitingSession);
    }

    #[test]
    fn start_session_quotes_arguments_and_exits_on_success() {
        let mut machine = Machine::new();
        create(&mut machine);
        response(&mut machine, Response::Success);

        let outputs = commands(
            &mut machine,
            Command::StartSession {
                exec: vec!["/usr/bin/sway".into(), "a b".into()],
                env: vec!["XDG_SESSION_TYPE=wayland".into()],
            },
        );
        assert_eq!(
            outputs,
            vec![Output::ToGreetd(Request::StartSession {
                cmd: vec!["'/usr/bin/sway'".into(), "'a b'".into()],
                env: vec!["XDG_SESSION_TYPE=wayland".into()],
            })]
        );
        assert_eq!(machine.phase(), Phase::Starting);

        let outputs = response(&mut machine, Response::Success);
        assert_eq!(outputs, vec![Output::ToUi(Event::Started), Output::Quit]);
    }

    #[test]
    fn a_cancel_ack_is_not_mistaken_for_an_auth_success() {
        let mut machine = Machine::new();
        create(&mut machine);
        commands(&mut machine, Command::CancelSession);
        assert_eq!(machine.phase(), Phase::Cancelling);

        let outputs = response(&mut machine, Response::Success);
        assert_eq!(outputs, vec![Output::ToUi(Event::Cancelled)]);
        assert_eq!(machine.phase(), Phase::Idle);
    }

    #[test]
    fn a_stray_success_does_not_start_a_session() {
        let mut machine = Machine::new();
        create(&mut machine);
        response(&mut machine, Response::Success);

        let outputs = response(&mut machine, Response::Success);
        assert!(outputs.is_empty());
        assert_eq!(machine.phase(), Phase::AwaitingSession);

        let outputs = commands(
            &mut machine,
            Command::StartSession {
                exec: vec!["/bin/true".into()],
                env: vec![],
            },
        );
        assert!(matches!(outputs[0], Output::ToGreetd(_)));
        assert_eq!(machine.phase(), Phase::Starting);
    }

    #[test]
    fn responding_without_a_prompt_is_rejected() {
        let mut machine = Machine::new();
        let outputs = commands(
            &mut machine,
            Command::Respond {
                value: "hunter2".into(),
            },
        );
        assert!(matches!(outputs[0], Output::ToUi(Event::Error { .. })));
    }

    #[test]
    fn transport_failure_quits() {
        let mut machine = Machine::new();
        let outputs = machine.handle_transport_failure("greetd closed the connection");
        assert_eq!(
            outputs,
            vec![
                Output::ToUi(Event::Exited {
                    reason: "greetd closed the connection".into(),
                }),
                Output::Quit,
            ]
        );
    }
}
