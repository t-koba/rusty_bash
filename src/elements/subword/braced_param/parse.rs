//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{ShellCore, Feeder};
use crate::elements::subscript::Subscript;
use crate::error::parse::ParseError;
use super::{BracedParam, Param};
use super::optional_operation;

impl BracedParam {
    fn eat_subscript(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> Result<bool, ParseError> {
        if let Some(s) = Subscript::parse(feeder, core)? {
            ans.text += &s.text;
            if s.text.contains('@') {
                ans.is_array = true;
            }
            ans.param.subscript = Some(s);
            return Ok(true);
        }

        Ok(false)
    }

    fn eat_param(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> bool {
        let len = feeder.scanner_name(core);
        if len != 0 {
            ans.param = Param{ name: feeder.consume(len), subscript: None};
            ans.text += &ans.param.name;
            return true;
        }

        let mut len = feeder.scanner_uint(core);
        if len == 0 {
            len = feeder.scanner_special_and_positional_param();
        }

        if len != 0 {
            ans.param = Param {name: feeder.consume(len), subscript: None};
            ans.is_array = ans.param.name == "@";
            ans.text += &ans.param.name;
            return true;
        }

        feeder.starts_with("}")
    }

    fn eat_unknown(feeder: &mut Feeder, ans: &mut Self, core: &mut ShellCore) -> Result<(), ParseError> {
        if feeder.len() == 0 {
            feeder.feed_additional_line(core)?;
        }

        let unknown = match feeder.starts_with("\\}") {
            true  => feeder.consume(2),
            false => {
                let len = feeder.scanner_char(); //feeder.nth(0).unwrap().len_utf8();
                feeder.consume(len)
            },
        };

        ans.unknown += &unknown.clone();
        ans.text += &unknown;
        Ok(())
    }

    pub fn parse(feeder: &mut Feeder, core: &mut ShellCore) -> Result<Option<Self>, ParseError> {
        if ! feeder.starts_with("${") {
            return Ok(None);
        }
        let mut ans = Self::default();
        ans.text += &feeder.consume(2);

        if feeder.starts_with("#") && ! feeder.starts_with("#}") {
            ans.num = true;
            ans.text += &feeder.consume(1);
        }else if feeder.starts_with("!") {
            ans.indirect = true;
            ans.text += &feeder.consume(1);
        }

        if Self::eat_param(feeder, &mut ans, core) {
            Self::eat_subscript(feeder, &mut ans, core)?;

            if let Some(op) = optional_operation::parse(feeder, core)? {
                ans.text += &op.get_text();
                ans.optional_operation = Some(op);
            }
        }
        while ! feeder.starts_with("}") {
            Self::eat_unknown(feeder, &mut ans, core)?;
        }

        ans.text += &feeder.consume(1);
        Ok(Some(ans))
    }
}
