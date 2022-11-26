//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::debuginfo::DebugInfo;
use crate::ShellCore;
use crate::Feeder;
//use crate::feeder::scanner::*;

use crate::abst_elems::ArgElem;

pub struct SubArgNonQuoted {
    pub text: String,
    pub pos: DebugInfo,
    pub is_value: bool,
}

impl ArgElem for SubArgNonQuoted {
    fn get_text(&self) -> String {
        self.text.clone()
    }

    fn eval(&mut self, _conf: &mut ShellCore) -> Vec<Vec<String>> {
        if self.is_value {
            vec!(vec!(self.text.clone()))
        }else{
            vec!(vec!(self.text.replace("\n", " ")))
        }
    }
}

impl SubArgNonQuoted {
    pub fn new(text: String, pos: DebugInfo, is_value: bool) -> SubArgNonQuoted {
        SubArgNonQuoted {
            text: text.clone(),
            pos: pos,
            is_value: is_value, 
        }
    }

    pub fn parse(text: &mut Feeder, is_in_brace: bool) -> Option<SubArgNonQuoted> {
        let pos = text.scanner_non_quoted_word(is_in_brace);
        if pos == 0{
            None
        }else{
            Some( SubArgNonQuoted::new(text.consume(pos), DebugInfo::init(text), false) )
        }
    }
}