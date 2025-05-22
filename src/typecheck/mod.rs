//! Hindley-Milner type inference 
//! resource: https://course.ccs.neu.edu/cs4410sp19/lec_type-inference_notes.html
//! 

use std::{collections::{HashMap, HashSet}, fmt::format};
use crate::ast::*;

pub enum Type {
    Int,
    Bool,
    Unit,
    Action,
    
    Fun(Vec<Type>, Box<Type>), // for instantiated type

    TypVar(String), 
}



impl Type {
    fn free_var(&self, typ_var_binded: &HashSet<String>) -> HashSet<String> {
        match self {
            Type::Int |
            Type::Bool |
            Type::Unit |
            Type::Action => HashSet::new(),
            Type::Fun(params, ret) => {
            let mut typ_var_binded: HashSet<String> = typ_var_binded.clone();
            let new_binds = params.iter()
            .filter_map(|typ| 
                if let Type::TypVar(name) = typ { Some(name.clone())}
                else { None }
            );
            typ_var_binded.extend(new_binds);
            
            ret.free_var(&typ_var_binded)
            },
            Type::TypVar(x) => {
                if typ_var_binded.contains(x) { HashSet::new() }
                else { HashSet::from([x.clone()]) }
            },
        }
    }
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
    pub typevar_id: u64,
    pub var_to_typ: HashMap<String, Type>,
    // pub var_to_typ_scheme: HashMap<String, TypeScheme>,
}

impl TypecheckEnv {
    fn gen_new_typevar(&mut self) -> Type {
        self.typevar_id += 1;
        let typevar_name = format!("tau{:?}", self.typevar_id);
        Type::TypVar(typevar_name)
    }

    fn apply_subst_typ(&self, typ: Type) {
        todo!()
    }

    // fn apply_subst_typscheme(&self, typ_scheme: TypeScheme) {
    //     todo!()
    // }

    fn apply_subst_typenv(&mut self, new_substs: Vec<(String, Type)>) {
        todo!()
    }


}
