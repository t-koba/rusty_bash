//SPDX-FileCopyrightText: 2023 Ryuichi Ueda <ryuichiueda@gmail.com>
//SPDX-FileCopyrightText: 2023 @caro@mi.shellgei.org
//SPDX-License-Identifier: BSD-3-Clause

use crate::ShellCore;
use crate::signal;
use crate::error::exec::ExecError;
use nix::sys::signal::Signal;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::{thread, time};
use signal_hook::iterator::Signals;

pub fn trap(core: &mut ShellCore, args: &mut Vec<String>) -> i32 {
    let forbiddens = Vec::from(signal_hook::consts::FORBIDDEN);
    let signals = match args_to_nums(&args[2..], &forbiddens){
        Ok(v) => v,
        Err(e) => {
            e.print(core);
            return 1;
        }
    };

    for n in &signals {
        let s: Signal = TryFrom::try_from(*n).unwrap();
        signal::ignore(s);
    }

    run_thread(signals, &args[1], core);

    0
}

fn run_thread(signal_nums: Vec<i32>, script: &String, core: &mut ShellCore) {
    core.trapped.push((Arc::new(AtomicBool::new(false)), script.clone()));

    let trap = Arc::clone(&core.trapped.last().unwrap().0);

    thread::spawn(move || {
        let mut signals = Signals::new(signal_nums.clone())
                          .expect("sush(fatal): cannot prepare signal data");

        loop {
            thread::sleep(time::Duration::from_millis(100));
            for signal in signals.pending() {
                if signal_nums.contains(&signal) {
                    trap.store(true, Relaxed);
                }
            }
        }
    });
}

fn arg_to_num(arg: &str, forbiddens: &Vec<i32>) -> Result<i32, ExecError> {
    if let Ok(n) = arg.parse::<i32>() {
        if forbiddens.contains(&n) {
            return Err(ExecError::Other(format!("trap: {}: forbidden signal for trap", arg)));
        }
        return Ok(n);
    }

    return Err(ExecError::Other(format!("trap: {}: invalid signal specification", arg)));
}

fn args_to_nums(args: &[String], forbiddens: &Vec<i32>) -> Result<Vec<i32>, ExecError> {
    let mut ans = vec![];
    for a in args {
        let n = arg_to_num(a, forbiddens)?;
        ans.push(n);
    }
    Ok(ans)
}
