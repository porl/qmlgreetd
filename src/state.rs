use std::collections::BTreeMap;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MAX_STATE_BYTES: u64 = 64 * 1024;
const MAX_FIELD_LEN: usize = 256;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_user: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub session_by_user: BTreeMap<String, String>,
}

impl State {
    pub fn load(path: &Path) -> Self {
        let Ok(metadata) = std::fs::metadata(path) else {
            return Self::default();
        };
        if !metadata.is_file() || metadata.len() > MAX_STATE_BYTES {
            return Self::default();
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(mut state) = serde_json::from_str::<Self>(&content) else {
            return Self::default();
        };
        state.sanitize();
        state
    }

    pub fn remember(&mut self, user: &str, session: Option<&str>) {
        self.last_user = Some(user.to_string());
        if let Some(session) = session {
            self.session_by_user
                .insert(user.to_string(), session.to_string());
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;

        let temporary = path.with_extension("json.tmp");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&json)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    }

    fn sanitize(&mut self) {
        if self.last_user.as_ref().is_some_and(|user| too_long(user)) {
            self.last_user = None;
        }
        self.session_by_user
            .retain(|user, session| !too_long(user) && !too_long(session));
    }
}

fn too_long(value: &str) -> bool {
    value.is_empty() || value.len() > MAX_FIELD_LEN
}

pub fn state_path() -> PathBuf {
    if let Some(path) = std::env::var_os("QMLGREETD_STATE") {
        return PathBuf::from(path);
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("qmlgreetd").join("state.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qmlgreetd-state-{}-{}-{:?}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = temp_dir("round-trip");
        let path = dir.join("state.json");

        let mut state = State::default();
        state.remember("alice", Some("sway"));
        state.remember("bob", None);
        state.save(&path).unwrap();

        let loaded = State::load(&path);
        assert_eq!(loaded.last_user.as_deref(), Some("bob"));
        assert_eq!(
            loaded.session_by_user.get("alice").map(String::as_str),
            Some("sway")
        );
        assert!(!loaded.session_by_user.contains_key("bob"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_is_atomic_and_private() {
        let dir = temp_dir("private");
        let path = dir.join("state.json");
        let mut state = State::default();
        state.remember("alice", Some("sway"));
        state.save(&path).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(dir_mode & 0o777, 0o700);
        assert!(!path.with_extension("json.tmp").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn malformed_state_falls_back_to_defaults() {
        let dir = temp_dir("malformed");
        let path = dir.join("state.json");
        std::fs::write(&path, b"not json at all").unwrap();
        assert_eq!(State::load(&path), State::default());

        std::fs::write(
            &path,
            b"{\"last_user\": \"a\", \"session_by_user\": {\"b\": 1}}",
        )
        .unwrap();
        assert_eq!(State::load(&path), State::default());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn oversized_state_is_ignored() {
        let dir = temp_dir("oversized");
        let path = dir.join("state.json");
        let big = format!(
            "{{\"last_user\":\"{}\"}}",
            "x".repeat(MAX_STATE_BYTES as usize)
        );
        std::fs::write(&path, big).unwrap();
        assert_eq!(State::load(&path), State::default());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn long_fields_are_dropped_on_load() {
        let dir = temp_dir("long-fields");
        let path = dir.join("state.json");
        let long = "x".repeat(MAX_FIELD_LEN + 1);
        std::fs::write(
            &path,
            format!("{{\"last_user\":\"{long}\",\"session_by_user\":{{\"{long}\":\"sway\"}}}}"),
        )
        .unwrap();

        let state = State::load(&path);
        assert_eq!(state.last_user, None);
        assert!(state.session_by_user.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
