//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::abst_script_elem::ScriptElem;
use nix::unistd::{Pid, fork, ForkResult, pipe, close, dup2};
use std::os::unix::prelude::RawFd;
use crate::elem_script::Script;
use std::process::exit;
use crate::utils::dup_and_close;
use crate::elem_redirect::Redirect;
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;
use crate::elem_end_of_command::Eoc;
use crate::elem_arg_delimiter::ArgDelimiter;

/* ( script ) */
pub struct CompoundParen {
    pub script: Option<Script>,
    pub redirects: Vec<Box<Redirect>>,
    pub text: String,
    pid: Option<Pid>, 
    pub pipein: RawFd,
    pub pipeout: RawFd,
    /* The followings are set by a pipeline or a com expansion. */
    pub expansion: bool,
    pub expansion_str: String,
    pub prevpipein: RawFd,
    pub eoc: Option<Eoc>,
}

impl ScriptElem for CompoundParen {
    fn exec(&mut self, conf: &mut ShellCore) -> Option<Pid>{
        if self.expansion {
            self.set_command_expansion_pipe();
        }

        unsafe {
            match fork() {
                Ok(ForkResult::Child) => {
                    if self.expansion {
                        dup_and_close(self.pipeout, 1);
                    }else{
                        self.set_child_io();
                    }
                    if let Some(s) = &mut self.script {
                        s.exec(conf);
                        exit(conf.vars["?"].parse::<i32>().unwrap());
                    };
                },
                Ok(ForkResult::Parent { child } ) => {
                    self.pid = Some(child);
                    return Some(child);
                },
                Err(err) => panic!("Failed to fork. {}", err),
            }
        }

        None
    }

    fn get_pid(&self) -> Option<Pid> { self.pid }

    fn set_pipe(&mut self, pin: RawFd, pout: RawFd, pprev: RawFd) {
        self.pipein = pin;
        self.pipeout = pout;
        self.prevpipein = pprev;
    }

    fn set_parent_io(&mut self) -> RawFd {
        if self.pipeout >= 0 {
            close(self.pipeout).expect("Cannot close outfd");
        }
        return self.pipein;
    }

    fn get_eoc_string(&mut self) -> String {
        if let Some(e) = &self.eoc {
            return e.text.clone();
        }

        "".to_string()
    }
}

impl CompoundParen {
    pub fn new() -> CompoundParen{
        CompoundParen {
            script: None,
            pid: None,
            redirects: vec!(),
            text: "".to_string(),
            pipein: -1,
            pipeout: -1,
            expansion: false,
            expansion_str: "".to_string(),
            prevpipein: -1,
            eoc: None,
        }
    }

    fn set_expansion(&mut self, pin: RawFd, pout: RawFd) {
        self.pipein = pin;
        self.pipeout = pout;
    }

    fn set_command_expansion_pipe(&mut self){
        let p = pipe().expect("Pipe cannot open");
        self.set_expansion(p.0, p.1);
    }

    pub fn parse(text: &mut Feeder, conf: &mut ShellCore) -> Option<CompoundParen> {
        if text.len() == 0 || text.nth(0) != '(' {
            return None;
        }

        let backup = text.clone();
        text.consume(1);
        let mut ans = CompoundParen::new();

        if let Some(s) = Script::parse(text, conf, true) {
            ans.text = "(".to_owned() + &s.text + ")";
            ans.script = Some(s);
        }

        if text.len() == 0 || text.nth(0) != ')' {
            text.rewind(backup);
            return None;
        }

        text.consume(1);

        loop {
            if let Some(d) = ArgDelimiter::parse(text){
                ans.text += &d.text;
            }

            if let Some(r) = Redirect::parse(text){
                    ans.text += &r.text;
                    ans.redirects.push(Box::new(r));
            }else{
                break;
            }
        }
        if let Some(e) = Eoc::parse(text){
            ans.text += &e.text;
            ans.eoc = Some(e);
        }

        Some(ans)
    }

    fn set_child_io(&mut self) {
        for r in &self.redirects {
            self.set_redirect(r);
        };

        //eprintln!("{} {} {}", self.pipein, self.pipeout, self.prevpipein);
        if self.pipein != -1 {
            close(self.pipein).expect("a");
        }
        if self.pipeout != -1 {
            dup_and_close(self.pipeout, 1);
        }

        if self.prevpipein != -1 {
            dup_and_close(self.prevpipein, 0);
        }

    }

    fn set_redirect_fds(&self, r: &Box<Redirect>){
        if let Ok(num) = r.path[1..].parse::<i32>(){
            dup2(num, r.left_fd).expect("Invalid fd");
        }else{
            panic!("Invalid fd number");
        }
    }

    fn set_redirect(&self, r: &Box<Redirect>){
        if r.path.len() == 0 {
            panic!("Invalid redirect");
        }

        if r.direction_str == ">" {
            if r.path.chars().nth(0) == Some('&') {
                self.set_redirect_fds(r);
                return;
            }

            if let Ok(file) = OpenOptions::new().truncate(true).write(true).create(true).open(&r.path){
                dup_and_close(file.into_raw_fd(), r.left_fd);
            }else{
                panic!("Cannot open the file: {}", r.path);
            };
        }else if r.direction_str == "&>" {
            if let Ok(file) = OpenOptions::new().truncate(true).write(true).create(true).open(&r.path){
                dup_and_close(file.into_raw_fd(), 1);
                dup2(1, 2).expect("Redirection error on &>");
            }else{
                panic!("Cannot open the file: {}", r.path);
            };
        }else if r.direction_str == "<" {
            if let Ok(file) = OpenOptions::new().read(true).open(&r.path){
                dup_and_close(file.into_raw_fd(), r.left_fd);
            }else{
                panic!("Cannot open the file: {}", r.path);
            };
        }
    }
}
