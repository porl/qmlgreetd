pub mod sessions;
pub mod users;

use serde::Serialize;

use crate::state::State;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub users: Vec<users::User>,
    pub sessions: Vec<sessions::Session>,
    pub state: State,
}
