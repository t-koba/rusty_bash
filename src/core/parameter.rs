//SPDXFileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDXLicense-Identifier: BSD-3-Clause

use crate::ShellCore;
use std::env;

impl ShellCore {
    pub fn get_param_ref(&mut self, key: &str) -> &str {
        if let Some(n) = self.get_position_param_pos(key) {
            return &self.position_parameters[n];
        }

        if  self.parameters.get(key) == None {
            if let Ok(val) = env::var(key) {
                self.set_param(key, &val);
            }
        }

        match self.parameters.get(key) {
            Some(val) => val,
            None      => "",
        }
    }

    fn get_position_param_pos(&self, key: &str) -> Option<usize> {
        if ! (key.len() == 1 && "0" <= key && key <= "9") {
            return None;
        }

        let n = key.parse::<usize>().unwrap();
        if n < self.position_parameters.len() {
            Some(n)
        }else{
            None
        }
    }

    pub fn set_param(&mut self, key: &str, val: &str) {
        match env::var(key) {
            Ok(_) => env::set_var(key, val),
            _     => {},
        }

        self.parameters.insert(key.to_string(), val.to_string());
    }
}
