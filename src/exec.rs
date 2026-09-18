#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecError {
    Empty,
    UnterminatedQuote,
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "the Exec line is empty"),
            Self::UnterminatedQuote => write!(f, "the Exec line has an unterminated quote"),
        }
    }
}

pub fn parse_exec(exec: &str) -> Result<Vec<String>, ExecError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut quote: Option<char> = None;
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else if c == '\\' && q == '"' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                } else if c == '%' {
                    expand_field_code(&mut chars, &mut current);
                } else {
                    current.push(c);
                }
            }
            None => match c {
                ' ' | '\t' => {
                    if has_token {
                        tokens.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                }
                '"' | '\'' => {
                    quote = Some(c);
                    has_token = true;
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                    has_token = true;
                }
                '%' => {
                    expand_field_code(&mut chars, &mut current);
                }
                _ => {
                    current.push(c);
                    has_token = true;
                }
            },
        }
    }

    if quote.is_some() {
        return Err(ExecError::UnterminatedQuote);
    }
    if has_token {
        tokens.push(current);
    }
    if tokens.is_empty() {
        return Err(ExecError::Empty);
    }
    Ok(tokens)
}

fn expand_field_code(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, current: &mut String) {
    match chars.next() {
        Some('%') => current.push('%'),
        Some(_) => {}
        None => current.push('%'),
    }
}

pub fn shell_quote(arg: &str) -> String {
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('\'');
    for c in arg.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

pub fn to_greetd_cmd(argv: &[String]) -> Vec<String> {
    argv.iter().map(|arg| shell_quote(arg)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(parse_exec("foo bar").unwrap(), vec!["foo", "bar"]);
    }

    #[test]
    fn respects_quotes() {
        assert_eq!(
            parse_exec("\"/usr/bin/foo bar\" --x").unwrap(),
            vec!["/usr/bin/foo bar", "--x"]
        );
        assert_eq!(parse_exec("'a b' c").unwrap(), vec!["a b", "c"]);
    }

    #[test]
    fn removes_field_codes() {
        assert_eq!(parse_exec("foo %U").unwrap(), vec!["foo"]);
        assert_eq!(parse_exec("--file=%f").unwrap(), vec!["--file="]);
        assert_eq!(parse_exec("100%%").unwrap(), vec!["100%"]);
        assert_eq!(parse_exec("%f").unwrap_err(), ExecError::Empty);
    }

    #[test]
    fn rejects_malformed_input() {
        assert_eq!(parse_exec("").unwrap_err(), ExecError::Empty);
        assert_eq!(
            parse_exec("foo \"bar").unwrap_err(),
            ExecError::UnterminatedQuote
        );
    }

    #[test]
    fn quotes_for_a_shell() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), r#"'it'\''s'"#);
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn hostile_exec_is_treated_literally() {
        let argv = parse_exec("echo safe; touch pwned").unwrap();
        assert_eq!(argv, vec!["echo", "safe;", "touch", "pwned"]);

        let command = format!("exec {}", to_greetd_cmd(&argv).join(" "));
        assert_eq!(command, "exec 'echo' 'safe;' 'touch' 'pwned'");

        let dir = std::env::temp_dir().join(format!(
            "qmlgreetd-exec-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&dir)
            .output()
            .unwrap();

        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "safe; touch pwned\n"
        );
        assert!(!dir.join("pwned").exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
