use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

/// The parts of a user AccountsService knows that /etc/passwd does not carry
/// usefully: the real name and the avatar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub username: String,
    pub real_name: String,
    pub icon_file: Option<PathBuf>,
}

const SERVICE: &str = "org.freedesktop.Accounts";
const MANAGER: &str = "/org/freedesktop/Accounts";
const USER_INTERFACE: &str = "org.freedesktop.Accounts.User";

/// Every user AccountsService has cached, or nothing when the service is not
/// on the bus (a console host, or accountsservice not installed). This is
/// enrichment only: who can log in comes from /etc/passwd.
pub fn list() -> Vec<Account> {
    let Some(json) = busctl(&[MANAGER, SERVICE, "ListCachedUsers"]) else {
        return Vec::new();
    };
    let Some(paths) = parse_cached_users(&json) else {
        return Vec::new();
    };
    paths
        .iter()
        .filter_map(|path| {
            busctl(&[
                path,
                "org.freedesktop.DBus.Properties",
                "GetAll",
                "s",
                USER_INTERFACE,
            ])
        })
        .filter_map(|json| parse_account(&json))
        .map(drop_unreadable_icon)
        .collect()
}

/// busctl rather than a D-Bus client library: it is always present with
/// systemd, and this is one call per user at greeter startup. The JSON mode is
/// locale-independent and needs systemd 257+; older versions fail the call and
/// the greeter runs without avatars.
fn busctl(method: &[&str]) -> Option<String> {
    let output = Command::new("busctl")
        .args(["--json=short", "call", SERVICE])
        .args(method)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// `busctl call … ListCachedUsers`:
/// {"type":"ao","data":[["/org/freedesktop/Accounts/User1000"]]}.
/// `data` is the list of return values; the method returns one array of paths.
pub fn parse_cached_users(json: &str) -> Option<Vec<String>> {
    let value: Value = serde_json::from_str(json).ok()?;
    let paths = value.get("data")?.as_array()?.first()?.as_array()?;
    Some(
        paths
            .iter()
            .filter_map(|path| path.as_str().map(str::to_string))
            .collect(),
    )
}

/// `busctl call … Properties.GetAll`:
/// {"type":"a{sv}","data":[{"UserName":{"type":"s","data":"porl"},…}]}.
/// A variant arrives as {"type": signature, "data": value}; the fields read
/// here are all strings (numbers stay numbers, as with Uid).
pub fn parse_account(json: &str) -> Option<Account> {
    let value: Value = serde_json::from_str(json).ok()?;
    let properties = value.get("data")?.as_array()?.first()?.as_object()?;

    let username = variant_string(properties.get("UserName")?)?;
    let real_name = properties
        .get("RealName")
        .and_then(variant_string)
        .unwrap_or_default();
    let icon_file = properties
        .get("IconFile")
        .and_then(variant_string)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);

    Some(Account {
        username,
        real_name,
        icon_file,
    })
}

fn variant_string(variant: &Value) -> Option<String> {
    variant.get("data")?.as_str().map(str::to_string)
}

/// AccountsService reports `$HOME/.face` as the icon file even when it does not
/// exist, and a path it cannot read looks like a broken image in the greeter.
fn drop_unreadable_icon(mut account: Account) -> Account {
    if let Some(icon) = &account.icon_file {
        if !icon.is_file() {
            account.icon_file = None;
        }
    }
    account
}

/// Overlays AccountsService's real name and avatar onto the passwd users, by
/// username. A user AccountsService does not know is left alone.
pub fn enrich(users: &mut [crate::enumerate::users::User], accounts: &[Account]) {
    for user in users.iter_mut() {
        let Some(account) = accounts.iter().find(|a| a.username == user.username) else {
            continue;
        };
        if !account.real_name.trim().is_empty() {
            user.display_name = account.real_name.clone();
        }
        if let Some(icon) = &account.icon_file {
            user.avatar = Some(icon.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enumerate::users::User;

    const CACHED_USERS: &str = r#"{"type":"ao","data":[["/org/freedesktop/Accounts/User1000","/org/freedesktop/Accounts/User1001"]]}"#;

    const PORL: &str = r#"{"type":"a{sv}","data":[{"Uid":{"type":"t","data":1000},"UserName":{"type":"s","data":"porl"},"RealName":{"type":"s","data":"Porl Smith"},"AccountType":{"type":"i","data":1},"IconFile":{"type":"s","data":"/var/lib/AccountsService/icons/porl"},"Shell":{"type":"s","data":"/run/current-system/sw/bin/fish"}}]}"#;

    fn user(username: &str, display_name: &str) -> User {
        User {
            username: username.to_string(),
            display_name: display_name.to_string(),
            uid: 1000,
            avatar: None,
        }
    }

    #[test]
    fn parses_cached_user_paths() {
        assert_eq!(
            parse_cached_users(CACHED_USERS),
            Some(vec![
                "/org/freedesktop/Accounts/User1000".to_string(),
                "/org/freedesktop/Accounts/User1001".to_string(),
            ])
        );
    }

    #[test]
    fn parses_an_account() {
        assert_eq!(
            parse_account(PORL),
            Some(Account {
                username: "porl".to_string(),
                real_name: "Porl Smith".to_string(),
                icon_file: Some(PathBuf::from("/var/lib/AccountsService/icons/porl")),
            })
        );
    }

    #[test]
    fn tolerates_missing_and_empty_fields() {
        let minimal = r#"{"type":"a{sv}","data":[{"UserName":{"type":"s","data":"bob"}}]}"#;
        assert_eq!(
            parse_account(minimal),
            Some(Account {
                username: "bob".to_string(),
                real_name: String::new(),
                icon_file: None,
            })
        );

        let no_icon = r#"{"type":"a{sv}","data":[{"UserName":{"type":"s","data":"bob"},"RealName":{"type":"s","data":""},"IconFile":{"type":"s","data":""}}]}"#;
        let account = parse_account(no_icon).unwrap();
        assert_eq!(account.real_name, "");
        assert_eq!(account.icon_file, None);
    }

    #[test]
    fn rejects_non_account_json() {
        assert_eq!(parse_account("{}"), None);
        assert_eq!(parse_account(r#"{"type":"a{sv}","data":[]}"#), None);
        assert_eq!(parse_account("Call failed: …"), None);
        assert_eq!(parse_cached_users(r#"{"type":"ao","data":[]}"#), None);
    }

    #[test]
    fn drops_icons_that_are_not_readable_files() {
        let existing = std::env::current_exe().unwrap();
        let kept = drop_unreadable_icon(Account {
            username: "porl".to_string(),
            real_name: String::new(),
            icon_file: Some(existing.clone()),
        });
        assert_eq!(kept.icon_file, Some(existing));

        let dropped = drop_unreadable_icon(Account {
            username: "porl".to_string(),
            real_name: String::new(),
            icon_file: Some(PathBuf::from("/nonexistent/.face")),
        });
        assert_eq!(dropped.icon_file, None);
    }

    #[test]
    fn enrich_overlays_real_name_and_avatar() {
        let mut users = vec![
            user("porl", "porl"),
            user("gloria", "Gloria"),
            user("other", "Other"),
        ];
        let accounts = vec![Account {
            username: "porl".to_string(),
            real_name: "Porl Smith".to_string(),
            icon_file: Some(PathBuf::from("/var/lib/AccountsService/icons/porl")),
        }];
        enrich(&mut users, &accounts);

        assert_eq!(users[0].display_name, "Porl Smith");
        assert_eq!(
            users[0].avatar,
            Some(PathBuf::from("/var/lib/AccountsService/icons/porl"))
        );
        // No account, or an empty real name: gecos stands.
        assert_eq!(users[1].display_name, "Gloria");
        assert_eq!(users[1].avatar, None);
        assert_eq!(users[2].display_name, "Other");
    }

    #[test]
    fn enrich_keeps_gecos_for_an_empty_real_name() {
        let mut users = vec![user("porl", "Porl Gecos")];
        let accounts = vec![Account {
            username: "porl".to_string(),
            real_name: "   ".to_string(),
            icon_file: None,
        }];
        enrich(&mut users, &accounts);
        assert_eq!(users[0].display_name, "Porl Gecos");
    }
}
