//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

mod bash_glob;
mod core;
mod term;
mod utils;
mod feeder;

use std::{env, process};

use crate::core::ShellCore;
use crate::feeder::Feeder;

fn show_version() {
    eprintln!("Rusty Bash, TERMINAL SKELETON");
    eprintln!("© 2022 Ryuichi Ueda");
    eprintln!("License: BSD 3-Clause\n");

    eprintln!("This is open source software. You can redistirbute and use in source\nand binary forms with or without modification under the license.");
    eprintln!("There is no warranty, to the extent permitted by law.");
    process::exit(0);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--version" {
        show_version();
    }

    /* Ignore Ctrl+C (Childlen will receive instead.) */
    ctrlc::set_handler(move || { })
    .expect("Unable to set the Ctrl+C handler.");

    let mut core = ShellCore::new();
    main_loop(&mut core);
}

fn main_loop(core: &mut ShellCore) {
    let mut feeder = Feeder::new();
    loop {
        if feeder.feed_line(core) {
            let txt = feeder.consume(feeder.remaining.len());
            if txt == "exit\n" {
                process::exit(0);
            }
            println!("{}", txt);
        }
    }
}
