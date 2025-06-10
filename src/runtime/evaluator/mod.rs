use crate::ast::{Assn, Expr, Prog, Service, Test, ReplCmd, Decl};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

mod eval_expr;
mod eval_stmt;
mod utils;

#[derive(Debug, Clone)]
pub enum Val {
    Number(i32),
    Bool(bool),
    Action(Vec<Assn>),
    Func(Vec<String>, Box<Expr>),
}

pub struct Evaluator {
    var_id_cnt: i32,
    pub reactive_names: HashSet<String>,

    // context are modally separated into, we didn't use lambda expr var
    // as we subst directly in place when we eval expr
    // reactive def/var name -> val
    pub reactive_name_to_vals: HashMap<String, Expr>,
    // lambda expr var name -> val
    // pub exprvar_name_to_val: HashMap<String, Expr>,
}

impl Evaluator {
    pub fn new(reactive_names_to_vals: HashMap<String, Expr>) -> Evaluator {
        Evaluator {
            var_id_cnt: 0,
            reactive_names: HashSet::new(),
            reactive_name_to_vals: reactive_names_to_vals,
        }
    }


    

}

pub fn eval_assns(assns: &mut Vec<Assn>, env: HashMap<String, Expr>) {
    let mut eval = Evaluator::new(env);
    for assn in assns.iter_mut() {
        eval.eval_assn(assn);
    }
}
pub fn eval_test(test: &Test, env: &HashMap<String, Expr>) -> Result<(), String> {
        
        let mut eval = Evaluator::new(env.clone());

        for cmd in &test.commands {
            match cmd {
                ReplCmd::Do(expr) => {
                    let mut expr_clone = expr.clone();
                    let _ = eval.eval_expr(&mut expr_clone);
                }
                ReplCmd::Assert(expr) => {
                    let mut expr_clone = expr.clone();
                    eval.eval_assert(&mut expr_clone)?;
                    match expr_clone {
                        Expr::Bool {val} => {
                            if !val {
                                return Err(format!("{} returned false", expr));
                            }
                            
                        }
                        _=> {
                            return Err(format!("Assert requires a boolean"))
                        }
                    }
                }
            }
        }
        Ok(())
    }

pub fn eval_srv(srv: &Service) -> Evaluator {
    let mut srv = srv.clone();
    let mut eval = Evaluator::new(HashMap::new());
    for decl in srv.decls.iter_mut() {
        eval.eval_decl(decl);
        println!("{}", decl);
    }
    eval
}

pub fn eval_prog(prog: &Prog) {
    let mut global_vals = HashMap::new();  // global vals from services
    for srv in prog.services.iter() {
        let srv_evaluator = eval_srv(srv);

        global_vals.extend(srv_evaluator.reactive_name_to_vals);   // adding service's vals to global
    }


    for test in prog.tests.iter() {
        if let Err(e) = eval_test(test, &global_vals) {
            println!("{} failed because {}", test.name,e);
            
        }
        else {
            println!("{} passed", test.name);
        }
    }
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
