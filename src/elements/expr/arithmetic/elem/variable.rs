//SPDX-FileCopyrightText: 2024 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use crate::{Feeder, ShellCore};
use crate::elements::expr::arithmetic::ArithmeticExpr;
use crate::error::exec::ExecError;
use crate::utils;
use crate::utils::exit;
use super::super::ArithElem;
use super::{float, int};

fn to_num(w: &str, sub: &String, core: &mut ShellCore) -> Result<ArithElem, ExecError> {
    if w.find('\'').is_some() {
        return Err(ExecError::OperandExpected(w.to_string()));
    }

    let name = w.to_string();//w.eval_as_value(core)?;
    str_to_num(&name, sub, core)
}

pub fn str_to_num(name: &str, sub: &String, 
                  core: &mut ShellCore) -> Result<ArithElem, ExecError> {
    let mut name = name.to_string();

    const RESOLVE_LIMIT: i32 = 10000;

    for i in 0..RESOLVE_LIMIT {
        match utils::is_name(&name, core) {
            //true  => name = get_param(&name, sub, core)?,//core.db.get_param(&name)?,
            true  => name = core.db.get_param2(&name, sub)?,//core.db.get_param(&name)?,
            false => {
                break
            },
        }

        if i == RESOLVE_LIMIT - 1 {
            return Err(ExecError::Recursion(name.clone()));
        }
    }

    match single_str_to_num(&name, core) {
        Ok(e)  => Ok(e),
        Err(_) => {
            resolve_arithmetic_op(&name, sub, core)
        },
    }
}

fn resolve_arithmetic_op(name: &str, sub: &String,
                         core: &mut ShellCore) -> Result<ArithElem, ExecError> {
    let mut f = Feeder::new(&name);
    let mut parsed = match ArithmeticExpr::parse_after_eval(&mut f, core, "") {
        Ok(Some(p)) => p,
        _    => return Err(ExecError::OperandExpected(name.to_string())),
    };

    if parsed.elements.len() == 1 { // In this case, the element is not changed by the evaluation.
        return Err(ExecError::OperandExpected(name.to_string()));
    }

    if let Ok(eval) = parsed.eval(core) {
        return single_str_to_num(&eval, core);
    }

    Err(ExecError::OperandExpected(name.to_string()))
}

fn single_str_to_num(name: &str, core: &mut ShellCore) -> Result<ArithElem, ExecError> {
    if name.contains('.') {
        let f = float::parse(&name)?;
        return Ok(ArithElem::Float(f));
    }

    if utils::is_name(&name, core) {
        return Ok( ArithElem::Integer(0) );
    }

    let n = int::parse(&name)?;
    Ok( ArithElem::Integer(n) )
}

pub fn set_and_to_value(name: &str, sub: &String, core: &mut ShellCore,
                        inc: i128, pre: bool) -> Result<ArithElem, ExecError> {
    //dbg!("{:?}", &name);
    //dbg!("{:?}", &sub);
    let num = str_to_num(&name, sub, core);
    //dbg!("{:?}", &num);

    match num {
        Ok(ArithElem::Integer(n))        => {
            if inc != 0 {
                //set_param(name, sub, &(n + inc).to_string(), core)?;
                core.db.set_param2(&name, sub, &(n + inc).to_string(), None)?;
                //core.db.set_param(name, &(n + inc).to_string(), None)?;
            }
            match pre {
                true  => Ok(ArithElem::Integer(n+inc)),
                false => Ok(ArithElem::Integer(n)),
            }
        },
        Ok(ArithElem::Float(n))        => {
            if inc != 0 {
                //set_param(name, sub, &(n + inc as f64).to_string(), core)?;
                core.db.set_param2(&name, sub, &(n + inc as f64).to_string(), None)?;
                //core.db.set_param(name, &(n + inc as f64).to_string(), None)?;
            }
            match pre {
                true  => Ok(ArithElem::Float(n+inc as f64)),
                false => Ok(ArithElem::Float(n)),
            }
        },
        Ok(_) => exit::internal("unknown element"),
        Err(err_msg) => return Err(err_msg), 
    }
}

pub fn get_sign(s: &mut String) -> String {
    *s = s.trim().to_string();
    match s.starts_with("+") || s.starts_with("-") {
        true  => {
            let c = s.remove(0).to_string();
            *s = s.trim().to_string();
            c
        },
        false => "+".to_string(),
    }
}

pub fn substitution(op: &str, stack: &mut Vec<ArithElem>, core: &mut ShellCore)
-> Result<(), ExecError> {
    let mut right = match stack.pop() {
        Some(mut e) => {e.change_to_value(0, core)?; e},
        _ => return Err(ExecError::OperandExpected(op.to_string())),
    };

    let ans = match stack.pop() {
        Some(ArithElem::Variable(w, s, 0)) => {
            let index = match s {
                Some(mut sub) => sub.eval(core, &w)?,
                None => "".to_string(),
            };
            subs(op, &w, &index, &mut right, core)?
        },
        Some(ArithElem::Variable(_, _, _)) => return Err(ExecError::AssignmentToNonVariable(op.to_string()) ),
        _ => return Err(ExecError::AssignmentToNonVariable(op.to_string()) ),
    };

    stack.push(ans);
    Ok(())
}

fn subs(op: &str, w: &str, sub: &String, right_value: &mut ArithElem, core: &mut ShellCore)
                                      -> Result<ArithElem, ExecError> {
    if w.find('\'').is_some() {
        return Err(ExecError::OperandExpected(w.to_string()));
    }

    let name = w.to_string();//w.eval_as_value(core)?;
    right_value.change_to_value(0, core)?; // InParen -> Value
    let right_str = right_value.to_string();

    match op {
        "=" => {
            core.db.set_param2(&name, sub, &right_str, None)?;
            return Ok(right_value.clone());
        },
        "+=" => {
            let mut val_str = core.db.get_param2(&name, sub)?;
            if val_str == "" {
                val_str = "0".to_string();
            }
            if let Ok(left) = val_str.parse::<i128>() {
                match right_value {
                    ArithElem::Integer(n) => {
                        core.db.set_param2(&name, sub, &(left + *n).to_string(), None)?;
                        return Ok(ArithElem::Integer(left + *n));
                    },
                    _ => {},
                }
            }else if let Ok(left) = val_str.parse::<f64>() {
                if let ArithElem::Float(f) = right_value {
                    core.db.set_param2(&name, sub, &(left + *f).to_string(), None)?;
                    return Ok(ArithElem::Float(left + *f));
                }
            }
        },
        _   => {},
    }

    match (to_num(w, sub, core)?, right_value) {
        (ArithElem::Integer(cur), ArithElem::Integer(right)) => Ok(int::substitute(op, &name, sub, cur, *right, core)?),
        (ArithElem::Float(cur), ArithElem::Integer(right)) => Ok(float::substitute(op, &name, sub, cur, *right as f64, core)?),
        (ArithElem::Float(cur), ArithElem::Float(right)) => Ok(float::substitute(op, &name, sub, cur, *right, core)?),
        (ArithElem::Integer(cur), ArithElem::Float(right)) => Ok(float::substitute(op, &name, sub, cur as f64, *right, core)?),
        _ => Err(ExecError::Other("not supported yet".to_string())),
    }
}

