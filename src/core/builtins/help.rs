//SPDX-FileCopyrightText: 2024 Ryuichi Ueda <ryuichiueda@gmail.com>
//SPDX-FileCopyrightText: 2023 @caro@mi.shellgei.org
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::core::builtins::error_;
use crate::utils;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HelpMode {
    Full,
    Short,
}

pub fn help(core: &mut ShellCore, args: &[String]) -> i32 {
    let mut mode = HelpMode::Full;
    let mut names = Vec::new();
    let mut parse_options = true;

    for arg in &args[1..] {
        if parse_options && arg == "--" {
            parse_options = false;
        } else if parse_options && arg.starts_with('-') && arg.len() > 1 {
            match arg.as_str() {
                "-d" | "-m" => mode = HelpMode::Full,
                "-s" => mode = HelpMode::Short,
                "--help" => {
                    print_help_usage();
                    return 0;
                }
                _ => {
                    let msg = format!("{arg}: invalid option");
                    return error_(2, "help", &msg, core);
                }
            }
        } else {
            names.push(arg.as_str());
        }
    }

    if names.is_empty() {
        print_help_index(core);
        return 0;
    }

    let mut status = 0;
    for name in names {
        if help_topic_exists(core, name) {
            print_help_topic(core, name, mode);
        } else {
            let msg = format!("no help topics match `{name}'.  Try `help help'.");
            status = error_(1, "help", &msg, core);
        }
    }

    status
}

fn help_topic_exists(core: &ShellCore, name: &str) -> bool {
    core.builtins.contains_key(name)
        || core.subst_builtins.contains_key(name)
        || utils::reserved(name)
}

fn print_help_index(core: &ShellCore) {
    println!("Shell builtins:");

    let mut names = core
        .builtins
        .keys()
        .chain(core.subst_builtins.keys())
        .map(String::as_str)
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();

    for chunk in names.chunks(4) {
        for name in chunk {
            print!("{name:<16}");
        }
        println!();
    }
}

fn print_help_topic(core: &ShellCore, name: &str, mode: HelpMode) {
    let description = help_description(core, name);
    match mode {
        HelpMode::Short => println!("{name}: {description}"),
        HelpMode::Full => {
            println!("{name}: {description}");
            println!("    This command is implemented by the shell.");
        }
    }
}

fn help_description(core: &ShellCore, name: &str) -> &'static str {
    if utils::reserved(name) {
        "shell keyword"
    } else if core.builtins.contains_key(name) || core.subst_builtins.contains_key(name) {
        "shell builtin"
    } else {
        "help topic"
    }
}

fn print_help_usage() {
    println!("help: help [-dms] [pattern ...]");
    println!("    Display information about builtin commands.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn help_reports_builtin_success() {
        let mut core = ShellCore::new();
        core.set_builtins();

        assert_eq!(help(&mut core, &args(&["help", "bind"])), 0);
    }

    #[test]
    fn help_reports_substitution_builtin_success() {
        let mut core = ShellCore::new();
        core.set_builtins();

        assert_eq!(help(&mut core, &args(&["help", "declare"])), 0);
    }

    #[test]
    fn help_reports_keyword_success() {
        let mut core = ShellCore::new();
        core.set_builtins();

        assert_eq!(help(&mut core, &args(&["help", "if"])), 0);
    }

    #[test]
    fn help_reports_missing_topic() {
        let mut core = ShellCore::new();
        core.set_builtins();

        assert_eq!(help(&mut core, &args(&["help", "not-a-builtin"])), 1);
    }

    #[test]
    fn help_reports_usage_errors() {
        let mut core = ShellCore::new();
        core.set_builtins();

        assert_eq!(help(&mut core, &args(&["help", "-z"])), 2);
    }
}
