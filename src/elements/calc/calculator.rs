//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use super::CalcElement;

fn op_order(op: &CalcElement) -> u8 {
    match op {
        CalcElement::UnaryOp(_) => 8,
        CalcElement::BinaryOp(s) => {
            match s.as_str() {
                "*" | "/" | "%" => 5, 
                "+" | "-"       => 4, 
                _ => 0,
            }
        },
        _ => 0, 
    }
}

fn to_string(op: &CalcElement) -> String {
    match op {
        CalcElement::Num(n) => n.to_string(),
        CalcElement::UnaryOp(s) => s.clone(),
        CalcElement::BinaryOp(s) => s.clone(),
        CalcElement::LeftParen => "(".to_string(),
        CalcElement::RightParen => ")".to_string(),
    }
}

fn rev_polish(elements: &Vec<CalcElement>) -> Result<Vec<CalcElement>, CalcElement> {
    let mut ans = vec![];
    let mut stack = vec![];
    let mut last = None;

    for e in elements {
        let ok = match e {
            CalcElement::LeftParen   => {stack.push(e.clone()); true},
            CalcElement::RightParen  => rev_polish_paren(&mut stack, &mut ans),
            CalcElement::Num(n)      => {ans.push(CalcElement::Num(*n)); true},
            CalcElement::UnaryOp(_)  => rev_polish_op(&e, &mut stack, &mut ans),
            CalcElement::BinaryOp(_) => rev_polish_op(&e, &mut stack, &mut ans),
        };

        if !ok 
        || (last == Some(CalcElement::LeftParen) && *e == CalcElement::RightParen) {
            return Err(e.clone());
        }

        last = Some(e.clone());
    }

    while stack.len() > 0 {
        ans.push(stack.pop().unwrap());
    }

    Ok(ans)
}

fn rev_polish_paren(stack: &mut Vec<CalcElement>, ans: &mut Vec<CalcElement>) -> bool {
    loop {
        match stack.last() {
            None => return false, 
            Some(CalcElement::LeftParen) => {
                stack.pop();
                return true;
            },
            Some(_) => ans.push(stack.pop().unwrap()),
        }
    }
}

fn rev_polish_op(cur_elem: &CalcElement,
                  stack: &mut Vec<CalcElement>, ans: &mut Vec<CalcElement>) -> bool {
    loop {
        match stack.last() {
            None | Some(CalcElement::LeftParen) => {
                stack.push(cur_elem.clone());
                break;
            },
            Some(_) => {
                let last = stack.last().unwrap();
                if op_order(last) <= op_order(cur_elem) {
                    stack.push(cur_elem.clone());
                    break;
                }
                ans.push(stack.pop().unwrap());
            },
        }
    }

    true
}

fn pop_operands(num: usize, stack: &mut Vec<CalcElement>) -> Vec<i64> {
    let mut ans = vec![];

    for _ in 0..num {
        let n = match stack.pop() {
            Some(CalcElement::Num(s)) => s,
            _ => return vec![],
        };
        ans.push(n);
    }

    ans
}

fn bin_operation(op: &str, stack: &mut Vec<CalcElement>) -> Result<(), String> {
    let operands = pop_operands(2, stack);
    if operands.len() != 2 {
        return Err("operand expected".to_string());
    }

    match op {
        "+" => stack.push( CalcElement::Num(operands[1] + operands[0]) ),
        "-" => stack.push( CalcElement::Num(operands[1] - operands[0]) ),
        "*" => stack.push( CalcElement::Num(operands[1] * operands[0]) ),
        "/" => stack.push( CalcElement::Num(operands[1] / operands[0]) ),
        _   => return Err("unexpected operator".to_string()),
    }

    Ok(())
}

fn unary_operation(op: &str, stack: &mut Vec<CalcElement>) -> Result<(), String> {
    let num = match stack.pop() {
        Some(CalcElement::Num(s)) => s,
        _ => return Err("operand expected".to_string()),
    };

    match op {
        "+" => stack.push( CalcElement::Num(num) ),
        "-" => stack.push( CalcElement::Num(-num) ),
        _ => return Err("unexpected operator".to_string()),
    }

    Ok(())
}


pub fn calculate(elements: &Vec<CalcElement>) -> Result<String, String> {
    if elements.len() == 0 {
        return Ok("0".to_string());
    }

    let rev_pol = match rev_polish(&elements) {
        Ok(ans) => ans,
        Err(e)  => {
            return Err(
                format!("syntax error: operand expected (error token is \"{}\")",
                        to_string(&e))
            );
        },
    };

    let mut stack = vec![];

    for e in rev_pol {
        let result = match e {
            CalcElement::Num(_) => {
                stack.push(e.clone());
                Ok(())
            },
            CalcElement::BinaryOp(ref op) => bin_operation(&op, &mut stack),
            CalcElement::UnaryOp(ref op) => unary_operation(&op, &mut stack),
            _ => Err("unknown operator".to_string()),
        };

        if let Err(err_msg) = result {
            return Err(
                format!("syntax error: {} (error token is \"{}\")",
                        err_msg, to_string(&e))
            );
        }
    }

    if stack.len() != 1 {
        return Err( format!("unknown syntax error",) );
    }

    match stack.pop() {
        Some(CalcElement::Num(n)) => Ok(n.to_string()),
        _ => Err( format!("unknown syntax error",) ),
    }
}