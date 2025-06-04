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


    // function to get declarations from service
    pub fn get_service_decls<'a>(&self, test: &Test, services: &'a [Service]) -> Result<&'a Vec<Decl>, String> {
        services.iter().find(|s| s.name == test.name).map(|s| &s.decls)
        .ok_or_else(|| format!("Service {} not found", test.name))
    }

    //loading service declarations
    pub fn load_service_decls(&mut self, decls: &[Decl], services: &[Service]) -> Result<(),String> {
        for decl in decls {
            match decl {
                Decl::VarDecl { name, val } => {
                    let val_expr = val.clone();
                    self.reactive_name_to_vals.insert(name.clone(), val_expr);
                    self.reactive_names.insert(name.clone());
                }
                Decl::DefDecl { name, val, is_pub } => {
                    let val_expr = val.clone();
                    self.reactive_name_to_vals.insert(name.clone(), val_expr);
                    self.reactive_names.insert(name.clone());
                }
                Decl::Import { srv_name } =>{   // not done yet

                }
            }
        }
        Ok(())
    }

    pub fn eval_test(&mut self, test: &Test, services: &[Service]) -> Result<(), String> {
        let declarations = self.get_service_decls(test, services)?; 

        self.load_service_decls(declarations, services)?;  //getting and loading declarations from services

        

        for cmd in &test.commands {
            match cmd {
                ReplCmd::Do(expr) => {
                    let mut expr_clone = expr.clone();
                    let _ = self.eval_expr(&mut expr_clone);
                }
                ReplCmd::Assert(expr) => {
                    let mut expr_clone = expr.clone();
                    self.eval_expr(&mut expr_clone)?;
                    match expr_clone {
                        Expr::Bool {val} => {
                            if !val {
                                println!("Assert failed");
                                return Err(format!("Assert Failed"));
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

}

pub fn eval_assns(assns: &mut Vec<Assn>, env: HashMap<String, Expr>) {
    let mut eval = Evaluator::new(env);
    for assn in assns.iter_mut() {
        eval.eval_assn(assn);
    }
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
    let mut global_decls = HashMap::new();  // global declarations from services
    for srv in prog.services.iter() {
        let srv_evaluator = eval_srv(srv);

        global_decls.extend(srv_evaluator.reactive_name_to_vals);   // adding service's decls to global
    }


    let mut test_eval = Evaluator::new(global_decls);  // using global decls as context
    for test in prog.tests.iter() {
        if let Err(e) = test_eval.eval_test(test, &prog.services) {
            println!("Test failed");
            println!("{}",e);
        }
        else {
            println!("Test passed");
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
