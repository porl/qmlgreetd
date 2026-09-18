use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    CreateSession {
        username: String,
    },
    PostAuthMessageResponse {
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<String>,
    },
    StartSession {
        cmd: Vec<String>,
        env: Vec<String>,
    },
    CancelSession,
}

impl Request {
    pub fn create_session(username: impl Into<String>) -> Self {
        Self::CreateSession {
            username: username.into(),
        }
    }

    pub fn response(value: impl Into<String>) -> Self {
        Self::PostAuthMessageResponse {
            response: Some(value.into()),
        }
    }

    pub fn acknowledge() -> Self {
        Self::PostAuthMessageResponse { response: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Success,
    Error {
        error_type: ErrorType,
        description: String,
    },
    AuthMessage {
        auth_message_type: AuthMessageType,
        auth_message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    AuthError,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMessageType {
    Secret,
    Visible,
    Info,
    Error,
}

impl AuthMessageType {
    pub fn expects_response(self) -> bool {
        matches!(self, Self::Secret | Self::Visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(request: &Request) -> String {
        serde_json::to_string(request).unwrap()
    }

    #[test]
    fn requests_use_the_wire_names() {
        assert_eq!(
            json(&Request::create_session("alice")),
            r#"{"type":"create_session","username":"alice"}"#
        );
        assert_eq!(
            json(&Request::response("hunter2")),
            r#"{"type":"post_auth_message_response","response":"hunter2"}"#
        );
        assert_eq!(
            json(&Request::acknowledge()),
            r#"{"type":"post_auth_message_response"}"#
        );
        assert_eq!(
            json(&Request::StartSession {
                cmd: vec!["/bin/true".into()],
                env: vec!["A=B".into()],
            }),
            r#"{"type":"start_session","cmd":["/bin/true"],"env":["A=B"]}"#
        );
        assert_eq!(
            json(&Request::CancelSession),
            r#"{"type":"cancel_session"}"#
        );
    }

    #[test]
    fn responses_parse_the_wire_names() {
        assert_eq!(
            serde_json::from_str::<Response>(r#"{"type":"success"}"#).unwrap(),
            Response::Success
        );
        assert_eq!(
            serde_json::from_str::<Response>(
                r#"{"type":"error","error_type":"auth_error","description":"nope"}"#
            )
            .unwrap(),
            Response::Error {
                error_type: ErrorType::AuthError,
                description: "nope".into(),
            }
        );
        assert_eq!(
            serde_json::from_str::<Response>(
                r#"{"type":"auth_message","auth_message_type":"secret","auth_message":"Password: "}"#
            )
            .unwrap(),
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                auth_message: "Password: ".into(),
            }
        );
    }

    #[test]
    fn only_secret_and_visible_expect_a_response() {
        assert!(AuthMessageType::Secret.expects_response());
        assert!(AuthMessageType::Visible.expects_response());
        assert!(!AuthMessageType::Info.expects_response());
        assert!(!AuthMessageType::Error.expects_response());
    }
}
