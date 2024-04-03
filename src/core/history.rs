//SPDXFileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDXLicense-Identifier: BSD-3-Clause

use crate::ShellCore;
use rev_lines::RevLines;
use std::fs::File;
use std::io::BufReader;

impl ShellCore {
    pub fn fetch_history(&mut self, pos: usize) -> String {
        if pos == 0 {
            return String::new();
        }

        let mut file_line = pos - 1;
        if let Ok(n) = self.get_param_ref("HISTFILESIZE").parse::<usize>() {
            file_line %= n;
        }

        if let Ok(hist_file) = File::open(self.get_param_ref("HISTFILE")){
            let mut rev_lines = RevLines::new(BufReader::new(hist_file));
            if let Some(Ok(s)) = rev_lines.nth(file_line) {
                return s;
            }
        }

        String::new()
    }
}
