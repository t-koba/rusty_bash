//SPDX-FileCopyrightText: 2022-2024 Ryuichi Ueda <ryuichiueda@gmail.com>
//SPDX-License-Identifier: BSD-3-Clause

use super::job::Job;
use crate::{Feeder, ShellCore};
use crate::utils::error;

enum Status{
    UnexpectedSymbol(String),
    NeedMoreLine,
    NormalEnd,
}

#[derive(Debug, Clone, Default)]
pub struct Script {
    pub jobs: Vec<Job>,
    pub job_ends: Vec<String>,
    text: String,
}

impl Script {
    pub fn exec(&mut self, core: &mut ShellCore) {
        for (job, end) in self.jobs.iter_mut().zip(self.job_ends.iter()) {
            if core.word_eval_error {
                return;
            }
            job.exec(core, end == "&");
        }
    }

    pub fn get_text(&self) -> String { self.text.clone() }

    fn eat_job(feeder: &mut Feeder, core: &mut ShellCore, ans: &mut Script) -> bool {
        if let Some(job) = Job::parse(feeder, core){
            ans.text += &job.text.clone();
            ans.jobs.push(job);
            true
        }else{
            false
        }
    }

    fn eat_job_end(feeder: &mut Feeder, ans: &mut Script) -> bool {
        if feeder.starts_with(";;") || feeder.starts_with(";&") {
            ans.job_ends.push("".to_string());
            return true;
        }
        let len = feeder.scanner_job_end();
        let end = &feeder.consume(len);
        ans.job_ends.push(end.clone());
        ans.text += &end;
        len != 0
    }

    fn check_nest(&self, feeder: &mut Feeder) -> Status {
        let nest = feeder.nest.last().unwrap();

        if nest.0 == "" && feeder.len() == 0 {
            return Status::NormalEnd;
        }

        match ( nest.1.iter().find(|e| feeder.starts_with(e)), self.jobs.len() ) {
            ( Some(end), 0 ) => return Status::UnexpectedSymbol(end.to_string()),
            ( Some(_), _)    => return Status::NormalEnd,
            ( None, _)       => {}, 
        }

        if feeder.len() > 0 {
            let remaining = feeder.consume(feeder.len());
            let first_token = remaining.split(" ").nth(0).unwrap().to_string();
            return Status::UnexpectedSymbol(first_token);
        }

        Status::NeedMoreLine
    }

    fn unalias(&mut self, core: &mut ShellCore) {
        for a in core.db.alias_memo.iter().rev() {
            self.text = self.text.replace(&a.1, &a.0);
        }

        core.db.alias_memo.clear();
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore,
                 permit_empty: bool) -> Option<Script> {
        let mut ans = Self::default();
        
        if permit_empty {
            ans.jobs.push(Job::default());
            ans.job_ends.push("".to_string());
        }

        loop {
            while Self::eat_job(feeder, core, &mut ans) 
               && Self::eat_job_end(feeder, &mut ans) {}

            match ans.check_nest(feeder){
                Status::NormalEnd => {
                    ans.unalias(core);
                    return Some(ans)
                },
                Status::UnexpectedSymbol(s) => {
                    core.db.set_param("LINENO", &feeder.lineno.to_string());
                    let s = format!("Unexpected token: {}", s);
                    error::print(&s, core);
                    core.db.set_param("?", "2");
                    break;
                },
                Status::NeedMoreLine => {
                    if ! feeder.feed_additional_line(core) {
                        break;
                    }
                },
            }
        }

        feeder.consume(feeder.len());
        core.db.alias_memo.clear();
        return None;
    }
}
