//SPDX-FileCopyrightText: 2022 Ryuichi Ueda ryuichiueda@gmail.com
//SPDX-License-Identifier: BSD-3-Clause

use super::pipeline::Pipeline;
use crate::{Feeder, ShellCore};

pub struct Job {
    pub pipelines: Vec<Pipeline>,
    pub text: String,
}

impl Job {
    pub fn exec(&mut self, core: &mut ShellCore) {
        for pipeline in self.pipelines.iter_mut() {
            pipeline.exec(core);
        }
    }

    pub fn parse(text: &mut Feeder, core: &mut ShellCore) -> Option<Job> {
        if let Some(pipeline) = Pipeline::parse(text, core){
            return Some( Job{text: pipeline.text.clone(), pipelines: vec!(pipeline)} );
        }
        None
    }
}