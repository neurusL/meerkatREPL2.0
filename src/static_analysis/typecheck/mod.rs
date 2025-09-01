//! Hindley-Milner type inference
//! resources:
//! https://course.ccs.neu.edu/cs4410sp19/lec_type-inference_notes.html,
//! our previous implementation
//! the union-find algorithm

mod tc_expr;
mod tc_srvs;
mod tc_stmt;
mod tc_test;
mod utils;

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::ast::Prog;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Bool,
    Unit,
    Action,

    Fun(Vec<Type>, Box<Type>), // for instantiated type

    TypVar(String),
}

/// Type Scheme represents polymorphic types,
/// e.g. \forall a, b, c in (a * b) -> c
// pub struct TypeScheme {
//     args: Vec<Type>,
//     body: Type,
// }

// impl TypeScheme {
//     fn free_var(&self) -> HashSet<String> {
//         todo!()
//     }
// }

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Bool => write!(f, "bool"),
            Type::Unit => write!(f, "unit"),
            Type::Action => write!(f, "action"),
            Type::Fun(args, ret) => {
                let joined = args
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect::<Vec<_>>()
                    .join(",");
                if args.len() > 1 {
                    write!(f, "({})->{}", joined, ret)
                } else {
                    write!(f, "{}->{}", joined, ret)
                }
            }
            Type::TypVar(name) => write!(f, "{}", name),
        }
    }
}

pub struct TypecheckEnv {
    // Expr::Var to type, todo: change this to more efficient stack of hashmap
    pub var_context: HashMap<String, Type>,

    pub open_service_name: Option<String>,
    pub services: HashMap<String, ServiceEnv>,

    // counter to generate new type var
    pub typevar_id: u64,
    // Type::var to type (canonical form)
    pub acc_subst: HashMap<String, Type>,
}

pub struct ServiceEnv {
    // reactive name to type
    pub name_context: HashMap<String, Type>,
    //pub var_to_typ_scheme: HashMap<String, TypeScheme>,
    pub imports: HashSet<String>,
}

impl Display for TypecheckEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (name, s)) in self.services.iter().enumerate() {
            if i != 0 {
                writeln!(f)?;
            }

            writeln!(f, "service: {name}")?;
            writeln!(f, "------------------")?;
            for (name, typ) in s.name_context.iter() {
                write!(f, "{}: {}\n", name, typ)?;
            }
            write!(f, "------------------")?;
        }

        Ok(())
    }
}

pub fn typecheck_prog(prog: &Prog) {
    let mut typ_env = TypecheckEnv::new();
    for srvs in prog.services.iter() {
        typ_env.typecheck_service(srvs);
    }

    println!("{}", typ_env);

    for test in prog.tests.iter() {
        typ_env.typecheck_test(test);
    }
}
