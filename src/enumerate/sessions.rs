use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::exec::parse_exec;

pub const DEFAULT_DATA_DIRS: &str = "/usr/local/share:/usr/share";
const SESSION_SUBDIR: &str = "wayland-sessions";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub exec: Vec<String>,
    pub desktop_names: Vec<String>,
    pub env: Vec<String>,
}

pub fn session_dirs(data_dirs: &str) -> Vec<PathBuf> {
    let data_dirs = if data_dirs.is_empty() {
        DEFAULT_DATA_DIRS
    } else {
        data_dirs
    };
    data_dirs
        .split(':')
        .filter(|entry| !entry.is_empty())
        .map(|entry| Path::new(entry).join(SESSION_SUBDIR))
        .collect()
}

pub fn load_sessions(dirs: &[PathBuf]) -> Vec<Session> {
    let mut sessions = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(session) = parse_desktop_entry(id, &content) {
                sessions.push(session);
            }
        }
    }
    sessions.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    sessions.dedup_by(|a, b| a.id == b.id);
    sessions
}

pub fn parse_desktop_entry(id: &str, content: &str) -> Option<Session> {
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut kind = None;
    let mut desktop_names = Vec::new();
    let mut hidden = false;
    let mut no_display = false;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.contains('[') {
            continue;
        }
        match key {
            "Name" => name = Some(value.to_string()),
            "Exec" => exec = Some(value.to_string()),
            "Type" => kind = Some(value.to_string()),
            "DesktopNames" => {
                desktop_names = value
                    .split(';')
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
            "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
            _ => {}
        }
    }

    if hidden || no_display {
        return None;
    }
    if kind.is_some_and(|kind| kind != "Application") {
        return None;
    }
    let name = name?;
    let exec = parse_exec(&exec?).ok()?;
    let env = session_env(id, &desktop_names);
    Some(Session {
        id: id.to_string(),
        name,
        exec,
        desktop_names,
        env,
    })
}

fn session_env(id: &str, desktop_names: &[String]) -> Vec<String> {
    let primary = desktop_names
        .first()
        .cloned()
        .unwrap_or_else(|| id.to_string());
    let current = if desktop_names.is_empty() {
        id.to_string()
    } else {
        desktop_names.join(":")
    };
    vec![
        "XDG_SESSION_TYPE=wayland".to_string(),
        format!("XDG_SESSION_DESKTOP={primary}"),
        format!("XDG_CURRENT_DESKTOP={current}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const SWAY: &str = "\
[Desktop Entry]
Name=Sway
Comment=A tiling compositor
Exec=sway --unsupported-gpu
Type=Application
DesktopNames=sway
";

    #[test]
    fn parses_a_wayland_session() {
        let session = parse_desktop_entry("sway", SWAY).unwrap();
        assert_eq!(session.id, "sway");
        assert_eq!(session.name, "Sway");
        assert_eq!(session.exec, vec!["sway", "--unsupported-gpu"]);
        assert_eq!(session.desktop_names, vec!["sway"]);
        assert_eq!(
            session.env,
            vec![
                "XDG_SESSION_TYPE=wayland",
                "XDG_SESSION_DESKTOP=sway",
                "XDG_CURRENT_DESKTOP=sway",
            ]
        );
    }

    #[test]
    fn session_env_falls_back_to_the_id_and_joins_desktop_names() {
        let no_names = parse_desktop_entry("foo", "[Desktop Entry]\nName=Foo\nExec=foo\n").unwrap();
        assert_eq!(
            no_names.env,
            vec![
                "XDG_SESSION_TYPE=wayland",
                "XDG_SESSION_DESKTOP=foo",
                "XDG_CURRENT_DESKTOP=foo",
            ]
        );

        let many = parse_desktop_entry(
            "bar",
            "[Desktop Entry]\nName=Bar\nExec=bar\nDesktopNames=Bar;Bar2\n",
        )
        .unwrap();
        assert_eq!(
            many.env,
            vec![
                "XDG_SESSION_TYPE=wayland",
                "XDG_SESSION_DESKTOP=Bar",
                "XDG_CURRENT_DESKTOP=Bar:Bar2",
            ]
        );
    }

    #[test]
    fn ignores_localized_names_and_other_groups() {
        let content = "\
[Desktop Entry]
Name=Base
Name[de]=Basis
Exec=foo

[Desktop Action New]
Name=New Window
Exec=foo --new
";
        let session = parse_desktop_entry("foo", content).unwrap();
        assert_eq!(session.name, "Base");
        assert_eq!(session.exec, vec!["foo"]);
    }

    #[test]
    fn skips_hidden_and_non_application_entries() {
        let hidden = "[Desktop Entry]\nName=X\nExec=x\nHidden=true\n";
        assert!(parse_desktop_entry("x", hidden).is_none());
        let link = "[Desktop Entry]\nName=X\nExec=x\nType=Link\n";
        assert!(parse_desktop_entry("x", link).is_none());
        let missing_exec = "[Desktop Entry]\nName=X\n";
        assert!(parse_desktop_entry("x", missing_exec).is_none());
    }

    #[test]
    fn builds_dirs_from_xdg_data_dirs() {
        assert_eq!(
            session_dirs("/run/current-system/sw/share:/usr/share"),
            vec![
                PathBuf::from("/run/current-system/sw/share/wayland-sessions"),
                PathBuf::from("/usr/share/wayland-sessions"),
            ]
        );
        assert_eq!(
            session_dirs(""),
            vec![
                PathBuf::from("/usr/local/share/wayland-sessions"),
                PathBuf::from("/usr/share/wayland-sessions"),
            ]
        );
    }

    #[test]
    fn loads_and_sorts_sessions_from_disk() {
        let dir = std::env::temp_dir().join(format!(
            "qmlgreetd-sessions-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("zebra.desktop"),
            "[Desktop Entry]\nName=Zebra\nExec=zebra\nType=Application\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("alpha.desktop"),
            "[Desktop Entry]\nName=Alpha\nExec=alpha\nType=Application\n",
        )
        .unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();

        let sessions = load_sessions(std::slice::from_ref(&dir));
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].name, "Alpha");
        assert_eq!(sessions[1].name, "Zebra");

        std::fs::remove_dir_all(&dir).ok();
    }
}
