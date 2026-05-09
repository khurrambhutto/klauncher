use crate::error::{AppError, AppResult};
use crate::platform::linux::desktop_entries::DesktopApp;
use std::process::Command;

pub fn launch_desktop_app(app: &DesktopApp) -> AppResult<()> {
    let argv = parse_exec_command(&app.exec);
    let Some((program, args)) = argv.split_first() else {
        return Err(AppError::InvalidDesktopEntry(format!(
            "Desktop entry has an empty Exec command: {}",
            app.name
        )));
    };

    Command::new(program).args(args).spawn()?;
    Ok(())
}

fn parse_exec_command(exec: &str) -> Vec<String> {
    let without_field_codes = remove_field_codes(exec);
    split_command_line(&without_field_codes)
}

fn remove_field_codes(exec: &str) -> String {
    let mut output = String::new();
    let mut chars = exec.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '%' {
            match chars.peek().copied() {
                Some('%') => {
                    output.push('%');
                    chars.next();
                }
                Some(code) if code.is_ascii_alphabetic() => {
                    chars.next();
                }
                _ => output.push(character),
            }
        } else {
            output.push(character);
        }
    }

    output
}

fn split_command_line(command: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut in_quotes = false;

    while let Some(character) = chars.next() {
        match character {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            character if character.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            }
            character => current.push(character),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
