//SPDX-FileCopyrightText: 2024 Ryuichi Ueda <ryuichiueda@gmail.com>
//SPDX-FileCopyrightText: 2023 @caro@mi.shellgei.org
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::core::builtins::error_;
use sushline::readline::{Config, Editor, History, Terminal};

pub fn bind(core: &mut ShellCore, args: &[String]) -> i32 {
    let line = core
        .readline
        .get_or_insert_with(|| Editor::new(Config::default(), Terminal::new(), History::new()));
    let bind_args = args[1..].iter().map(String::as_str).collect::<Vec<_>>();
    match line.bind_api().apply_builtin_args(&bind_args) {
        Ok(output) => {
            print!("{output}");
            0
        }
        Err(err) => {
            let status = if bind_error_is_usage(&err.message) {
                2
            } else {
                1
            };
            error_(status, "bind", &err.message, core)
        }
    }
}

fn bind_error_is_usage(message: &str) -> bool {
    message.contains("invalid option")
        || message.contains("option requires an argument")
        || message.contains("invalid keymap name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sushline::readline::BindQuery;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn bind_builtin_updates_persistent_readline_keymap() {
        let mut core = ShellCore::new();

        assert_eq!(
            bind(&mut core, &args(&["bind", "\"\\t\": menu-complete"])),
            0
        );

        let output = core
            .readline
            .as_mut()
            .unwrap()
            .bind_api()
            .print(BindQuery::QueryFunction("menu-complete".to_string()));
        assert!(output.contains("\"\\C-i\""), "{output}");
    }

    #[test]
    fn bind_builtin_can_unbind_existing_readline_key() {
        let mut core = ShellCore::new();

        assert_eq!(bind(&mut core, &args(&["bind", "-r", "\"\\C-a\""])), 0);

        let output = core
            .readline
            .as_mut()
            .unwrap()
            .bind_api()
            .print(BindQuery::QueryFunction("beginning-of-line".to_string()));
        assert!(!output.contains("\"\\C-a\""), "{output}");
    }

    #[test]
    fn bind_builtin_reports_usage_errors() {
        let mut core = ShellCore::new();

        assert_eq!(bind(&mut core, &args(&["bind", "-q"])), 2);
    }
}
