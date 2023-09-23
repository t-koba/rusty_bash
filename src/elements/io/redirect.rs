//SPDX-FileCopyrightText: 2023 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use std::fs::{File, OpenOptions};
use std::os::fd::{IntoRawFd, RawFd};
use std::io::Error;
use crate::elements::io;
use crate::{Feeder, ShellCore};

#[derive(Debug)]
pub struct Redirect {
    pub text: String,
    pub symbol: String,
    pub right: String,
    left_fd: RawFd,
    left_backup: RawFd,
}

impl Redirect {
    pub fn connect(&mut self, restore: bool) -> bool {
        let result = match self.symbol.as_str() {
            "<" => self.redirect_simple_input(restore),
            ">" => self.redirect_simple_output(restore),
            ">>" => self.redirect_append(restore),
            _ => panic!("SUSH INTERNAL ERROR (Unknown redirect symbol)"),
        };

        if ! result {
            eprintln!("bash: {}: {}", &self.right, Error::last_os_error().kind());
        }

        result
    }

    fn redirect_simple_input(&mut self, restore: bool) -> bool {
        if restore {
            self.left_fd = 0;
            self.left_backup = io::backup(0);
        }
        if let Ok(fd) = File::open(&self.right) {
            io::replace(fd.into_raw_fd(), 0);
            true
        }else{
            false
        }
    }

    fn redirect_simple_output(&mut self, restore: bool) -> bool {
        if restore {
            self.left_fd = 1;
            self.left_backup = io::backup(1);
        }
        if let Ok(fd) = File::create(&self.right) {
            io::replace(fd.into_raw_fd(), 1);
            true
        }else{
            false
        }
    }

    fn redirect_append(&mut self, restore: bool) -> bool {
        if restore {
            self.left_fd = 1;
            self.left_backup = io::backup(1);
        }
        if let Ok(fd) = OpenOptions::new().create(true).write(true)
                        .append(true).open(&self.right) {
            io::replace(fd.into_raw_fd(), 1);
            true
        }else{
            false
        }
    }

    pub fn restore(&mut self) {
        if self.left_backup >= 0 && self.left_fd >= 0 {
            io::replace(self.left_backup, self.left_fd);
        }
    }

    pub fn new() -> Redirect {
        Redirect {
            text: String::new(),
            symbol: String::new(),
            right: String::new(),
            left_fd: -1,
            left_backup: -1,
        }
    }

    fn eat_symbol(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_redirect_symbol(core);
        ans.symbol = feeder.consume(len);
        ans.text += &ans.symbol.clone();
        len != 0
    }

    fn eat_right(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let blank_len = feeder.scanner_blank(core);
        ans.text += &feeder.consume(blank_len);

        let len = feeder.scanner_word(core);
        ans.right = feeder.consume(len);
        ans.text += &ans.right.clone();
        len != 0
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Option<Redirect> {
        let mut ans = Self::new();

        if Self::eat_symbol(feeder, &mut ans, core) &&
           Self::eat_right(feeder, &mut ans, core) {
            Some(ans)
        }else{
            None
        }
    }
}