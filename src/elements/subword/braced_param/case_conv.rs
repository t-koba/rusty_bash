//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{Feeder, ShellCore};
use crate::elements::subword::braced_param::Word;
use crate::error::exec::ExecError;
use crate::error::parse::ParseError;
use crate::utils::glob;
use super::BracedParam;

#[derive(Debug, Clone, Default)]
pub struct CaseConv {
    pub all_replace: bool,
    pub pattern: Option<Word>,
    pub to_upper: bool,
}

impl CaseConv {
    fn to_string(&self, w: &Option<Word>, core: &mut ShellCore) -> Result<String, ExecError> {
        if let Some(w) = &w {
            match w.eval_for_case_word(core) {
                Some(s) => return Ok(s),
                None => match w.subwords.len() {
                    0 => return Ok("".to_string()),
                    _ => return Err(ExecError::Other("parse error".to_string())),
                },
            }
        }

        Ok("".to_string())
    }

    pub fn get_text(&self, text: &String, core: &mut ShellCore) -> Result<String, ExecError> {
        let tmp = self.to_string(&self.pattern, core)?;
        let extglob = core.shopts.query("extglob");
        let pattern = glob::parse(&tmp, extglob);

        let mut start = 0;
        let mut ans = String::new();
        let mut skip = 0;
        for ch in text.chars() {
            if skip > 0 {
                skip -= 1;
                start += ch.len_utf8();
                continue;
            }
    
            let len = glob::longest_match_length(&text[start..].to_string(), &pattern);
            if (len != 0 || pattern.is_empty()) && ! self.all_replace {
                if 'a' <= ch && ch <= 'z' {
                    let s = ch.to_string();
                    let ch = s.to_uppercase();
                    return Ok([&text[..start], &ch, &text[start+len..] ].concat());
                }
                return Ok(text.to_string());
            }

            if len != 0 {
                skip = text[start..start+len].chars().count() - 1;
            }
    
            if (len != 0 || pattern.is_empty()) && 'a' <= ch && ch <= 'z' {
                let s = ch.to_string();
                let ch = s.to_uppercase();
                ans += &ch;
            }else{
                ans += &ch.to_string();
            }
            start += ch.len_utf8();
        }
        Ok(ans)
    }

    pub fn eat(feeder: &mut Feeder, ans: &mut BracedParam, core: &mut ShellCore)
           -> Result<bool, ParseError> {
        if ! feeder.starts_with("^") && ! feeder.starts_with(",") {
            return Ok(false);
        }

        let mut info = CaseConv::default();

        if feeder.starts_with("^^") {
            info.to_upper = true;
            info.all_replace = true;
            ans.text += &feeder.consume(2);
        }else if feeder.starts_with("^") {
            info.to_upper = true;
            ans.text += &feeder.consume(1);
        }else if feeder.starts_with(",,") {
            info.all_replace = true;
            ans.text += &feeder.consume(2);
        }else if feeder.starts_with(",") {
            ans.text += &feeder.consume(1);
        }

        info.pattern = Some(BracedParam::eat_subwords(feeder, ans, vec!["}"], core)? );
        ans.case_conv = Some(info);
        return Ok(true);
    }
}
