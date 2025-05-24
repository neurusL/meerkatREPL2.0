use std::{collections::{HashMap, HashSet}, fmt::Display};
use crate::ast::{Assn, Expr, Prog};

mod utils;
mod eval_expr;
mod eval_stmt;

#[derive(Debug, Clone)] 
pub enum Val {
    Number(i32),
    Bool(bool),
    Action(Vec<Assn>),
    Func(Vec<String>, Box<Expr>),
}

impl From<Val> for Expr {
    fn from(val: Val) -> Self {
        match val {
            Val::Number(i) => Expr::Number { val: i },
            Val::Bool(b) => Expr::Bool { val: b },
            Val::Action(assns) => Expr::Action { assns },
            Val::Func(params, body) => Expr::Func { params, body },
        }
    }
}

impl Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Expr::from(self.clone()))
    }
}


pub struct Evaluator {
    pub var_id_cnt: i32,
    pub reactive_names: HashSet<String>,
    pub env: HashMap<String, Expr>,
}

impl Evaluator {
    pub fn new(reactive_names: HashSet<String>) -> Evaluator {
        Evaluator { 
            var_id_cnt: 0,
            reactive_names,
            env: HashMap::new(),
        }
    }
}

pub fn eval_prog(prog: &Prog) {
    let mut prog = prog.clone();
    for srvs in prog.services.iter_mut() {
        let mut eval = Evaluator::new(HashSet::new());
        for decl in srvs.decls.iter_mut() {
            eval.eval_decl(decl);
            println!("{}", decl);
        }
    }
}