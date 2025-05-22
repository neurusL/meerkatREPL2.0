//! Hindley-Milner type inference 
//! resources: 
//! https://course.ccs.neu.edu/cs4410sp19/lec_type-inference_notes.html,
//! our previous implementation
//! the union-find algorithm

mod utils;
mod tc_expr;

use std::{collections::{HashMap, HashSet}, iter::zip};


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


pub struct TypecheckEnv {
    
    pub var_context: HashMap<String, Type>, // Expr::Var to type, todo: change this to more efficient stack of hashmap
    // pub var_to_typ_scheme: HashMap<String, TypeScheme>,

    // counter to generate new type var
    pub typevar_id: u64,
    // Type::var to type (canonical form)
    pub acc_subst: HashMap<String, Type> 
    
}

