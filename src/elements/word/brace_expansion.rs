//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::elements::subword::Subword;
use crate::elements::word::Word;
use crate::elements::subword::single_quoted::SingleQuoted;

enum BraceType {
    Comma,
    Range,
}

fn after_dollar(s: &str) -> bool {
    s == "$" || s == "$$"
}

fn num_to_subword(n: i32) -> Box<dyn Subword> {
    Box::new( SingleQuoted { text: format!("'{}'", n)  } )
}

fn ascii_to_subword(c: char) -> Box<dyn Subword> {
    let table = vec!["^?", "\\M-^@", "\\M-^A", "\\M-^B", "\\M-^C", "\\M-^D", "\\M-^E", "\\M-^F", "\\M-^G", "\\M-^H", "\\M-\t", "\\M-\n", "\\M-^K", "\\M-^L", "\\M-^M", "\\M-^N", "\\M-^O", "\\M-^P", "\\M-^Q", "\\M-^R", "\\M-^S", "\\M-^T", "\\M-^U", "\\M-^V", "\\M-^W", "\\M-^X", "\\M-^Y", "\\M-^Z", "\\M-^[", "\\M-^\\", "\\M-^]", "\\M-^^", "\\M-^_", " ", "¡"];

    let n = c as usize;
    let text = if n >= 127 && n < 127 + table.len() {
        table[n-127].to_string()
    }else{
        c.to_string()
    };

    Box::new( SingleQuoted { text: format!("'{}'", text)  } )
}

pub fn eval(word: &mut Word) -> Vec<Word> {
    invalidate_brace(&mut word.subwords);

    let mut skip_until = 0;
    for i in word.scan_pos("{") {
        if i < skip_until { //ブレース展開の終わりまで処理をスキップ
             continue;
        }

        if let Some(d) = parse(&word.subwords[i..]) {
            let shift_d: Vec<usize> = d.0.iter().map(|e| e+i).collect();

            if i > 0 && after_dollar(word.subwords[i-1].get_text()) {
                skip_until = *shift_d.last().unwrap();
                continue;
            }

            return match d.1 {
                BraceType::Comma => expand_comma_brace(&word.subwords, &shift_d),
                BraceType::Range => expand_range_brace(&mut word.subwords, &shift_d),
            }
        }
    }

    vec![word.clone()]
}

fn invalidate_brace(subwords: &mut Vec<Box<dyn Subword>>) {
    if subwords.len() < 2 {
        return;
    }

    if subwords[0].get_text() == "{"
    && subwords[1].get_text() == "}" {
        subwords.remove(1);
        subwords[0].set_text("{}");
    }
}

fn parse(subwords: &[Box<dyn Subword>]) -> Option<(Vec<usize>, BraceType)> {
    let mut stack = vec![];
    for sw in subwords {
        stack.push(Some(sw.get_text()));
        if sw.get_text() == "}" {
            match get_delimiters(&mut stack) {
                Some(found) => return Some(found),
                _           => {},
            }
        }
    }
    None
}

fn get_delimiters(stack: &mut Vec<Option<&str>>) -> Option<(Vec<usize>, BraceType)> {
    let mut comma_pos = vec![];
    let mut period_pos = vec![];

    for i in (1..stack.len()-1).rev() {
        if stack[i] == Some(",") {
            comma_pos.push(i);
        } else if stack[i] == Some(".") {
            period_pos.push(i);
        }else if stack[i] == Some("{") { // find an inner brace expcomma_posion
            stack[i..].iter_mut().for_each(|e| *e = None);
            return None;
        }
    }

    if comma_pos.len() > 0 {
        comma_pos.reverse();
        comma_pos.insert(0, 0); // add "{" pos
        comma_pos.push(stack.len()-1); // add "}" pos
        return Some( (comma_pos, BraceType::Comma) );
    }

    if period_pos.len() > 1 && period_pos[0] == period_pos[1] + 1 {
        period_pos.reverse();
        period_pos.insert(0, 0);
        period_pos.push(stack.len()-1);
        return Some( (period_pos, BraceType::Range) );
    }
    None
}

fn comma_brace_to_subwords(subwords: &Vec<Box<dyn Subword>>, delimiters: &Vec<usize>)
                           -> Vec<Vec<Box<dyn Subword>>> {
    let mut ans = vec![];
    let mut from = delimiters[0] + 1;
    for to in &delimiters[1..] {
        ans.push(subwords[from..*to].to_vec());
        from = *to + 1;
    }
    ans
}

fn expand_comma_brace(subwords: &Vec<Box<dyn Subword>>, delimiters: &Vec<usize>) -> Vec<Word> {
    let left = subwords[..delimiters[0]].to_vec();
    let mut right = subwords[(delimiters.last().unwrap()+1)..].to_vec();
    invalidate_brace(&mut right);

    let sws = comma_brace_to_subwords(subwords, delimiters);
    let mut ws = subword_sets_to_words(&sws, &left, &right);

    let mut ans = vec![];
    for w in ws.iter_mut() {
        ans.append(&mut eval(w));
    }
    ans
}

fn expand_range_brace(subwords: &mut Vec<Box<dyn Subword>>, delimiters: &Vec<usize>) -> Vec<Word> {
    let start_wrap = subwords[delimiters[0]+1].make_unquoted_string(); // right of {
    let end_wrap = subwords[delimiters[delimiters.len()-1]-1].make_unquoted_string(); // left of }
    
    let (start, end) = match (start_wrap, end_wrap) {
        ( Some(s), Some(e) ) => (s, e),
        _ => return subwords_to_word(subwords),
    };

    let mut series = gen_nums(&start, &end);
    if series.len() == 0 {
        series = gen_chars(&start, &end);
    }
    if series.len() == 0 {
        return subwords_to_word(subwords);
    }
    let mut series2 = vec![];
    for e in series {
        series2.push(vec![e]);
    }

    let left = &subwords[..delimiters[0]];
    let mut right = subwords[(delimiters.last().unwrap()+1)..].to_vec();
    invalidate_brace(&mut right);

    subword_sets_to_words(&series2, left, &right)
}

fn gen_nums(start: &str, end: &str) -> Vec<Box<dyn Subword>> {
    let (start_num, end_num) = match (start.parse::<i32>(), end.parse::<i32>() ) {
        ( Ok(s), Ok(e) ) => (s, e),
        _ => return vec![],
    };

    let min = std::cmp::min(start_num, end_num);
    let max = std::cmp::max(start_num, end_num);

    let mut ans: Vec<Box<dyn Subword>> = (min..(max+1)).map(|n| num_to_subword(n) ).collect();
    if start_num > end_num {
        ans.reverse();
    }
    ans
}

fn gen_chars(start: &str, end: &str) -> Vec<Box<dyn Subword>> {
    let (start_num, end_num) = match (start.chars().nth(0), end.chars().nth(0) ) {
        ( Some(s), Some(e) ) => (s, e),
        _ => return vec![],
    };

    if start.chars().count() > 1 || end.chars().count() > 1 {
        return vec![];
    }

    let min = std::cmp::min(start_num, end_num);
    let max = std::cmp::max(start_num, end_num);

    let mut ans: Vec<Box<dyn Subword>> = (min..max).map(|n| ascii_to_subword(n) ).collect();
    ans.push( ascii_to_subword(max) );
    if start_num > end_num {
        ans.reverse();
    }
    ans
}

fn subword_sets_to_words(series: &Vec<Vec<Box<dyn Subword>>>,
                     left: &[Box<dyn Subword>], right: &[Box<dyn Subword>]) -> Vec<Word> {
    let mut ans = vec![];
    for sws in series {
        let mut w = Word::new();
        w.subwords = [ left, sws, right ].concat();
        w.text = w.subwords.iter().map(|s| s.get_text()).collect();
        ans.push(w);
    }
    ans
}

fn subwords_to_word(subwords: &Vec<Box<dyn Subword>>) -> Vec<Word> {
    let mut w = Word::new();
    w.subwords = subwords.to_vec();
    w.text = w.subwords.iter().map(|s| s.get_text()).collect();
    vec![w]
}
