use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswdEntry {
    pub username: String,
    pub uid: u32,
    pub gecos: String,
    pub shell: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct User {
    pub username: String,
    pub display_name: String,
    pub uid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<PathBuf>,
}

pub fn parse_passwd(content: &str) -> Vec<PasswdEntry> {
    content.lines().filter_map(parse_passwd_line).collect()
}

fn parse_passwd_line(line: &str) -> Option<PasswdEntry> {
    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 7 {
        return None;
    }
    let uid = fields[2].parse().ok()?;
    Some(PasswdEntry {
        username: fields[0].to_string(),
        uid,
        gecos: fields[4].to_string(),
        shell: fields[6].to_string(),
    })
}

pub fn parse_uid_min(content: &str) -> Option<u32> {
    content.lines().find_map(|line| {
        let value = line.trim().strip_prefix("UID_MIN")?.trim();
        value.parse().ok()
    })
}

pub fn is_human(entry: &PasswdEntry, uid_min: u32) -> bool {
    entry.uid >= uid_min && !entry.username.is_empty() && is_login_shell(&entry.shell)
}

fn is_login_shell(shell: &str) -> bool {
    if shell.is_empty() {
        return false;
    }
    let name = shell.rsplit('/').next().unwrap_or(shell);
    !matches!(name, "nologin" | "false" | "sync" | "shutdown" | "halt")
}

pub fn display_name(entry: &PasswdEntry) -> String {
    let name = entry.gecos.split(',').next().unwrap_or("").trim();
    if name.is_empty() {
        entry.username.clone()
    } else {
        name.to_string()
    }
}

pub fn list_users(entries: &[PasswdEntry], uid_min: u32) -> Vec<User> {
    let mut users: Vec<User> = entries
        .iter()
        .filter(|entry| is_human(entry, uid_min))
        .map(|entry| User {
            username: entry.username.clone(),
            display_name: display_name(entry),
            uid: entry.uid,
            avatar: None,
        })
        .collect();
    users.sort_by(|a, b| a.username.cmp(&b.username));
    users
}

pub fn read_passwd() -> Vec<PasswdEntry> {
    if let Ok(output) = Command::new("getent").arg("passwd").output() {
        if output.status.success() {
            return parse_passwd(&String::from_utf8_lossy(&output.stdout));
        }
    }
    std::fs::read_to_string("/etc/passwd")
        .map(|content| parse_passwd(&content))
        .unwrap_or_default()
}

pub fn uid_min() -> u32 {
    std::fs::read_to_string("/etc/login.defs")
        .ok()
        .and_then(|content| parse_uid_min(&content))
        .unwrap_or(1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "\
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
alice:x:1000:1000:Alice Doe,,,:/home/alice:/bin/bash
bob:x:1001:1001::/home/bob:/bin/sh
svc:x:999:999:Service:/var/lib/svc:/usr/bin/false
";

    #[test]
    fn parses_passwd_lines() {
        let entries = parse_passwd(PASSWD);
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[2].username, "alice");
        assert_eq!(entries[2].uid, 1000);
        assert_eq!(entries[2].gecos, "Alice Doe,,,");
        assert_eq!(entries[2].shell, "/bin/bash");
    }

    #[test]
    fn reads_uid_min_from_login_defs() {
        assert_eq!(
            parse_uid_min("# comment\nUID_MIN 5000\nUID_MAX 60000\n"),
            Some(5000)
        );
        assert_eq!(parse_uid_min("UID_MAX 60000\n"), None);
    }

    #[test]
    fn filters_to_human_accounts() {
        let users = list_users(&parse_passwd(PASSWD), 1000);
        let names: Vec<&str> = users.iter().map(|user| user.username.as_str()).collect();
        assert_eq!(names, vec!["alice", "bob"]);
        assert_eq!(users[0].display_name, "Alice Doe");
        assert_eq!(users[1].display_name, "bob");
    }

    #[test]
    fn rejects_system_and_non_login_shells() {
        let entries = parse_passwd(PASSWD);
        assert!(!is_human(&entries[0], 1000));
        assert!(!is_human(&entries[1], 1000));
        assert!(!is_human(&entries[4], 1000));
        assert!(is_human(&entries[2], 1000));
    }
}
