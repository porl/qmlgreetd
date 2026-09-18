use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    CreateSession { username: String },
    Respond { value: String },
    StartSession { exec: Vec<String>, env: Vec<String> },
    CancelSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Connected,
    AuthMessage { kind: AuthMessageKind, text: String },
    AuthError { text: String },
    Error { text: String },
    Authenticated,
    Started,
    Cancelled,
    Exited { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMessageKind {
    Secret,
    Visible,
    Info,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_and_events_use_the_documented_shape() {
        assert_eq!(
            serde_json::to_string(&Command::CreateSession {
                username: "alice".into()
            })
            .unwrap(),
            r#"{"cmd":"create_session","username":"alice"}"#
        );
        assert_eq!(
            serde_json::to_string(&Command::Respond {
                value: "hunter2".into()
            })
            .unwrap(),
            r#"{"cmd":"respond","value":"hunter2"}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::AuthMessage {
                kind: AuthMessageKind::Secret,
                text: "Password: ".into(),
            })
            .unwrap(),
            r#"{"event":"auth_message","kind":"secret","text":"Password: "}"#
        );
        assert_eq!(
            serde_json::to_string(&Event::Authenticated).unwrap(),
            r#"{"event":"authenticated"}"#
        );
    }
}
