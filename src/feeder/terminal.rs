//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-FileCopyrightText: 2026 @caro@mi.shellgei.org
//SPDX-License-Identifier: BSD-3-Clause

mod completion;
mod history;
mod prompt;

use crate::ShellCore;
use crate::error::input::InputError;
use history::{history_for_readline, remove_current_history_entry, update_current_history_entry};
use prompt::make_prompt_string;
use std::sync::atomic::Ordering::Relaxed;
use sushline::readline::{
    CommandContext, CompletionRequest, CompletionResponse, Edit, Editor, HistoryExpansion,
    HistoryExpansionPolicy, Hooks, Prompt, ReadlineResult, Terminal, expand_history,
};

struct SushHooks<'a> {
    core: &'a mut ShellCore,
}

impl Hooks for SushHooks<'_> {
    fn check_signals(&mut self) -> Option<i32> {
        if self.core.sigint.load(Relaxed) {
            return Some(libc::SIGINT);
        }
        self.core
            .trapped
            .iter()
            .any(|(flag, _)| flag.load(Relaxed))
            .then_some(libc::SIGINT)
    }

    fn version(&mut self) -> Option<String> {
        Some(format!(
            "Sushi shell (a.k.a. Sush), version {}",
            env!("CARGO_PKG_VERSION")
        ))
    }

    fn expand_history(
        &mut self,
        context: sushline::readline::HistoryExpansionContext<'_>,
    ) -> Result<HistoryExpansion, String> {
        let policy = HistoryExpansionPolicy {
            quotes_inhibit_expansion: true,
            ..HistoryExpansionPolicy::default()
        };
        expand_history(
            context.line,
            context.history,
            context.histchars,
            &policy,
            |_| false,
        )
        .map(|line| HistoryExpansion {
            line,
            print_only: false,
        })
        .map_err(|err| err.message())
    }

    fn complete(&mut self, request: CompletionRequest) -> Option<CompletionResponse> {
        completion::programmable_completion(self.core, &request)
    }

    fn default_complete(&mut self, request: &CompletionRequest) -> Option<CompletionResponse> {
        completion::default_complete(self.core, request)
    }

    fn command_names(&mut self) -> Vec<Vec<u8>> {
        completion::command_names(self.core)
    }

    fn variable_names(&mut self) -> Vec<Vec<u8>> {
        completion::variable_names(self.core)
    }

    fn shell_words(&mut self, line: &[u8]) -> Option<Vec<Vec<u8>>> {
        Some(completion::tokenize(line))
    }

    fn completion_word_breaks(&mut self) -> Option<Vec<u8>> {
        Some(completion::completion_word_breaks())
    }

    fn on_command(&mut self, context: CommandContext<'_>) -> Option<Edit> {
        run_readline_application_command(self.core, context)
    }
}

fn run_readline_application_command(
    core: &mut ShellCore,
    context: CommandContext<'_>,
) -> Option<Edit> {
    let line = String::from_utf8_lossy(context.line).into_owned();
    let _ = core.db.set_param("READLINE_LINE", &line, None);
    let _ = core
        .db
        .set_param("READLINE_POINT", &context.point.to_string(), None);
    let _ = core.db.set_param(
        "READLINE_MARK",
        &context.mark.unwrap_or(context.point).to_string(),
        None,
    );
    if let Some(argument) = context.argument {
        let _ = core
            .db
            .set_param("READLINE_ARGUMENT", &argument.to_string(), None);
    } else {
        let _ = core.db.set_param("READLINE_ARGUMENT", "", None);
    }

    let mut feeder = crate::Feeder::new(context.command);
    match crate::Script::parse(&mut feeder, core, false) {
        Ok(Some(mut script)) => {
            let _ = script.exec(core);
        }
        Ok(None) => {}
        Err(err) => err.print(core),
    }

    let line = core.db.get_param("READLINE_LINE").ok()?.into_bytes();
    let point = core
        .db
        .get_param("READLINE_POINT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let mark = core
        .db
        .get_param("READLINE_MARK")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());

    Some(Edit {
        line: Some(line),
        point,
        mark: Some(mark),
    })
}

pub fn read_line(core: &mut ShellCore, prompt: &str) -> Result<String, InputError> {
    let extend_history_entry = prompt == "PS2" && !core.history.is_empty();
    let prompt = make_prompt_string(core, prompt);
    let history = history_for_readline(core);
    if !extend_history_entry {
        core.history.insert(0, String::new());
    }

    let mut line = core.readline.take().unwrap_or_else(|| {
        Editor::new(
            sushline::readline::Config::default(),
            Terminal::new(),
            sushline::readline::History::new(),
        )
    });
    *line.history_mut() = history;

    let result = {
        let mut hooks = SushHooks { core };
        line.read_line(Prompt::new(prompt), &mut hooks)
    };
    core.readline = Some(line);

    match result {
        Ok(ReadlineResult::Line(bytes)) => match readline_bytes_to_feeder_line(bytes) {
            Ok((input, history)) => {
                update_current_history_entry(core, extend_history_entry, &history);
                Ok(input)
            }
            Err(e) => {
                remove_current_history_entry(core);
                Err(e)
            }
        },
        Ok(ReadlineResult::Interrupted) => {
            core.sigint.store(true, Relaxed);
            remove_current_history_entry(core);
            Err(InputError::Interrupt)
        }
        Ok(ReadlineResult::Eof) => {
            remove_current_history_entry(core);
            Err(InputError::Eof)
        }
        Err(_) => {
            remove_current_history_entry(core);
            Err(InputError::Eof)
        }
    }
}

fn readline_bytes_to_feeder_line(bytes: Vec<u8>) -> Result<(String, String), InputError> {
    let mut input = String::from_utf8(bytes).map_err(|_| InputError::NotUtf8)?;
    let history = input.trim_end().to_string();
    input.push('\n');
    Ok((input, history))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sushline::readline::KeyMapName;

    #[test]
    fn accepted_readline_line_keeps_feeder_trailing_newline() {
        let (input, history) =
            readline_bytes_to_feeder_line(b"echo \"alpha".to_vec()).expect("valid utf-8");
        assert_eq!(input, "echo \"alpha\n");
        assert_eq!(history, "echo \"alpha");
    }

    #[test]
    fn readline_application_command_updates_line_from_readline_variables() {
        let mut core = ShellCore::new();
        core.configure_c_mode().unwrap();
        let context = CommandContext {
            command: "READLINE_LINE=rewritten; READLINE_POINT=3; READLINE_MARK=1",
            line: b"original",
            point: 8,
            mark: None,
            argument: Some(2),
            key: b"\x0f",
            keymap: KeyMapName::Emacs,
        };

        let edit = run_readline_application_command(&mut core, context).unwrap();

        assert_eq!(edit.line, Some(b"rewritten".to_vec()));
        assert_eq!(edit.point, Some(3));
        assert_eq!(edit.mark, Some(Some(1)));
        assert_eq!(core.db.get_param("READLINE_ARGUMENT").unwrap(), "2");
    }
}
