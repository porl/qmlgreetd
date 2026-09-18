use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PowerOff,
    Reboot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Disabled,
    Mock,
    Systemctl,
}

pub fn parse_action(value: &str) -> Option<Action> {
    match value {
        "off" | "poweroff" | "shutdown" => Some(Action::PowerOff),
        "reboot" | "restart" => Some(Action::Reboot),
        _ => None,
    }
}

pub fn parse_mode(value: Option<&str>) -> Mode {
    match value {
        Some("mock") => Mode::Mock,
        Some("systemctl") => Mode::Systemctl,
        _ => Mode::Disabled,
    }
}

pub fn command(action: Action) -> [&'static str; 2] {
    match action {
        Action::PowerOff => ["systemctl", "poweroff"],
        Action::Reboot => ["systemctl", "reboot"],
    }
}

pub fn run(action: Action, mode: Mode) -> Result<(), String> {
    match mode {
        Mode::Disabled => Err("power actions are disabled".to_string()),
        Mode::Mock => Ok(()),
        Mode::Systemctl => {
            let [program, argument] = command(action);
            let status = Command::new(program)
                .arg(argument)
                .status()
                .map_err(|error| error.to_string())?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("{program} {argument} failed"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_actions_and_modes() {
        assert_eq!(parse_action("off"), Some(Action::PowerOff));
        assert_eq!(parse_action("shutdown"), Some(Action::PowerOff));
        assert_eq!(parse_action("reboot"), Some(Action::Reboot));
        assert_eq!(parse_action("sleep"), None);

        assert_eq!(parse_mode(Some("mock")), Mode::Mock);
        assert_eq!(parse_mode(Some("systemctl")), Mode::Systemctl);
        assert_eq!(parse_mode(None), Mode::Disabled);
        assert_eq!(parse_mode(Some("something-else")), Mode::Disabled);
    }

    #[test]
    fn maps_actions_to_systemctl() {
        assert_eq!(command(Action::PowerOff), ["systemctl", "poweroff"]);
        assert_eq!(command(Action::Reboot), ["systemctl", "reboot"]);
    }

    #[test]
    fn disabled_mode_refuses() {
        assert!(run(Action::PowerOff, Mode::Disabled).is_err());
    }

    #[test]
    fn mock_mode_is_inert() {
        assert!(run(Action::PowerOff, Mode::Mock).is_ok());
        assert!(run(Action::Reboot, Mode::Mock).is_ok());
    }
}
