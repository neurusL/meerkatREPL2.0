use super::Type;
use super::TypecheckEnv;
use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    iter::zip,
};

impl Type {
    fn free_var(&self, typ_var_binded: &HashSet<String>) -> HashSet<String> {
        match self {
            Type::Int | Type::Bool | Type::String | Type::Unit | Type::Action => HashSet::new(),
            // Calculate free type var in function type
            // e.g. (a, int, bool) -> b has free_var
            // for convenience, we clone the whole type bindings from previous
            // level, if perfomance matters, we can switch to a stack of type
            // binds to efficiently maintain type bindings
            Type::Fun(params, ret) => {
                let mut typ_var_binded: HashSet<String> = typ_var_binded.clone();
                let new_binds = params.iter().filter_map(|typ| {
                    if let Type::TypVar(name) = typ {
                        Some(name.clone())
                    } else {
                        None
                    }
                });
                typ_var_binded.extend(new_binds);
                ret.free_var(&typ_var_binded)
            }
            Type::TypVar(x) => {
                if typ_var_binded.contains(x) {
                    HashSet::new()
                } else {
                    HashSet::from([x.clone()])
                }
            }
        }
    }
}

impl TypecheckEnv {
    pub fn new(initial_context: HashMap<String, Type>) -> TypecheckEnv {
        // passing initial context as parameter (useful when tests need service's context)
        TypecheckEnv {
            var_context: initial_context, // using it as var context
            typevar_id: 0,
            acc_subst: HashMap::new(),
            table_context: HashMap::new()
        }
    }

    pub fn gen_typevar(&mut self) -> Type {
        self.typevar_id += 1;
        let typevar_name = format!("tau{:?}", self.typevar_id);
        self.acc_subst
            .insert(typevar_name.clone(), Type::TypVar(typevar_name.clone()));

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

    // union-find based unification
    pub fn find(&self, typ: &Type) -> Type {
        match typ {
            Type::Int | Type::Bool | Type::String | Type::Unit | Type::Action | Type::Fun(_, _) => typ.clone(),

            Type::TypVar(name) => {
                let canonical_typ = self
                    .acc_subst
                    .get(name)
                    .expect(&format!("cannot find {:?} in accumulated subst map", name));

                if canonical_typ != typ {
                    self.find(canonical_typ)
                } else {
                    // we reach the canonical form of type
                    canonical_typ.clone()
                }
            }
        }
    }

    pub fn unify(&mut self, typ1: &Type, typ2: &Type) -> bool {
        match (typ1, typ2) {
            (Type::Int, Type::Int)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Unit, Type::Unit)
            | (Type::Action, Type::Action) => true,

            (Type::Fun(args1, ret_typ1), Type::Fun(args2, ret_typ2)) => {
                if args1.len() != args2.len() {
                    return false;
                }
                zip(args1, args2).all(|(typ1, typ2)| self.unify(typ1, typ2))
                    && self.unify(ret_typ1, ret_typ2)
            }

            (Type::TypVar(_), _) | (_, Type::TypVar(_)) => {
                let cano_typ1 = self.find(typ1);
                let cano_typ2 = self.find(typ2);

                if let Type::TypVar(subst_by_name1) = cano_typ1 {
                    self.acc_subst.insert(subst_by_name1, cano_typ2);
                    true
                } else if let Type::TypVar(subst_by_name2) = cano_typ2 {
                    self.acc_subst.insert(subst_by_name2, cano_typ1);
                    true
                } else {
                    self.unify(&cano_typ1, &cano_typ2)
                }
            }

            _ => false,
        }
    }
}

impl Default for TypecheckEnv {
    fn default() -> Self {
        Self::new(HashMap::new()) // initializing with empty hashmap
    }
}
