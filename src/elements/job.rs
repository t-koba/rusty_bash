//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use super::pipeline::Pipeline;
use crate::{proc_ctrl, Feeder, ShellCore};
use crate::core::jobtable::JobEntry;
use crate::Script;
use crate::utils::exit;
use crate::error::exec::ExecError;
use crate::error::parse::ParseError;
use nix::sys::wait::WaitStatus;
use nix::unistd;
use nix::unistd::{Pid, ForkResult};
use std::sync::atomic::Ordering::Relaxed;

#[derive(Debug, Clone, Default)]
pub struct Job {
    pub pipelines: Vec<Pipeline>,
    pub pipeline_ends: Vec<String>,
    pub text: String,
}

impl Job {
    pub fn exec(&mut self, core: &mut ShellCore, bg: bool) -> Result<(), ExecError> {
        let pgid = match core.is_subshell {
            true  => unistd::getpgrp(),
            false => Pid::from_raw(0),
        };

        match bg {
            true  => self.exec_bg(core, pgid),
            false => self.exec_fg(core, pgid)?,
        }
        Ok(())
    }

    fn exec_fg(&mut self, core: &mut ShellCore, pgid: Pid) -> Result<(), ExecError> {
        let mut do_next = true;
        let susp_e_option = core.suspend_e_option;

        Self::check_trap(core);

        for (pipeline, end) in self.pipelines.iter_mut().zip(self.pipeline_ends.iter()) {

            core.suspend_e_option = susp_e_option || end == "&&" || end == "||";
            if do_next {
                core.jobtable_check_status()?;
                let (pids, exclamation, time, err) = pipeline.exec(core, pgid);
                let waitstatuses = proc_ctrl::wait_pipeline(core, pids.clone(), exclamation, time);

                Self::check_stop(core, &pipeline.text, &pids, &waitstatuses);

                if err.is_some() {
                    return Err(err.unwrap());
                }
            }

            do_next = (core.db.exit_status == 0) == (end == "&&");
            Self::check_trap(core);
        }
        Self::check_trap(core);
        Ok(())
    }

    pub fn check_trap(core: &mut ShellCore) {
        let bkup = core.db.exit_status;

        let mut scripts = vec![];
        for t in &core.trapped {
            if t.0.load(Relaxed) {
                scripts.push(t.1.clone());
                t.0.store(false, Relaxed);
            }
        }

        for s in scripts {
            let mut feeder = Feeder::new(&s);
            let mut script = match Script::parse(&mut feeder, core, true) {
                Ok(None) => {continue;},
                Ok(s) => s.unwrap(),
                Err(e) => {e.print(core); continue;},
            };

            if let Err(e) = script.exec(core) {
                e.print(core);
            }
        }

        core.db.exit_status = bkup;
    }

    fn check_stop(core: &mut ShellCore, text: &str,
                  pids: &Vec<Option<Pid>>, waitstatuses: &Vec<WaitStatus>) {
        if core.is_subshell || pids.is_empty() || pids[0] == None {
            return;
        }

        for ws in waitstatuses {
            if let WaitStatus::Stopped(_, _) = ws {
                let new_job_id = core.generate_new_job_id();
                let job = JobEntry::new(pids.to_vec(), &waitstatuses, &text, "Stopped", new_job_id); 
                core.job_table_priority.insert(0, new_job_id);
                core.job_table.push(job);
                return;
            }
        }
    }

    fn exec_bg(&mut self, core: &mut ShellCore, pgid: Pid) {
        let backup = match core.tty_fd.as_ref() {
            Some(fd) => Some(fd.try_clone().unwrap()),
            _ => None,
        };
        core.tty_fd = None;

        let pids = if self.pipelines.len() == 1 {
            if self.pipelines[0].commands.len() == 1 {
                self.pipelines[0].commands[0].set_force_fork();
            }
            self.pipelines[0].exec(core, pgid).0
        }else{
            match self.exec_fork_bg(core, pgid) {
                Ok(pid) => vec![pid],
                Err(e) => {
                    e.print(core);
                    return;
                },
            }
            //vec![self.exec_fork_bg(core, pgid)]
        };
        eprintln!("{}", &pids[0].unwrap().as_raw());
        let len = pids.len();
        let new_job_id = core.generate_new_job_id();
        core.job_table_priority.insert(0, new_job_id);
        core.job_table.push(JobEntry::new(pids, &vec![ WaitStatus::StillAlive; len ],
                &self.text, "Running", new_job_id));

        core.tty_fd = backup;
    }

    fn exec_fork_bg(&mut self, core: &mut ShellCore, pgid: Pid) -> Result<Option<Pid>, ExecError> {
        match unsafe{unistd::fork()? } {
            ForkResult::Child => {
                core.initialize_as_subshell(Pid::from_raw(0), pgid);
                if let Err(e) = self.exec(core, false) {
                    e.print(core);
                }
                exit::normal(core)
            },
            ForkResult::Parent { child } => {
                proc_ctrl::set_pgid(core, child, pgid);
                Ok(Some(child))
            },
        }
    }

    fn eat_blank_line(feeder: &mut Feeder, ans: &mut Job, core: &mut ShellCore) -> bool {
        let num = feeder.scanner_blank(core);
        ans.text += &feeder.consume(num);
        let com_num = feeder.scanner_comment();
        ans.text += &feeder.consume(com_num);
        if feeder.starts_with("\n") {
            ans.text += &feeder.consume(1);
            true
        }else{
            false
        }
    }

    fn eat_pipeline(feeder: &mut Feeder, ans: &mut Job, core: &mut ShellCore)
        -> Result<bool, ParseError> {
        if let Some(pipeline) = Pipeline::parse(feeder, core)? {
                ans.text += &pipeline.text.clone();
                ans.pipelines.push(pipeline);
                return Ok(true);
        }
        Ok(false)
    }

    fn eat_and_or(feeder: &mut Feeder, ans: &mut Job, core: &mut ShellCore) -> bool {
        let num = feeder.scanner_and_or(core);
        let end = feeder.consume(num);
        ans.pipeline_ends.push(end.clone());
        ans.text += &end;
        num != 0 //記号なしの場合にfalseが返る
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Result<Option<Job>, ParseError> {
        let mut ans = Self::default();
        while Self::eat_blank_line(feeder, &mut ans, core) {} 
        if ! Self::eat_pipeline(feeder, &mut ans, core)? {
            if ans.text.is_empty() {
                return Ok(None);
            }else{
                return Ok(Some(ans));
            }
        }

        while Self::eat_and_or(feeder, &mut ans, core) { 
            loop {
                while Self::eat_blank_line(feeder, &mut ans, core) {} 
                if Self::eat_pipeline(feeder, &mut ans, core)? {
                    break;  
                }
                if feeder.len() == 0 {
                    feeder.feed_additional_line(core)?;
                }
            }
        }

        let com_num = feeder.scanner_comment();
        ans.text += &feeder.consume(com_num);

        match ans.pipelines.len() > 0 {
            true  => Ok(Some(ans)),
            false => Ok(None),
        }
    }
}
