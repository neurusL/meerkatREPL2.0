use crate::ast::{Assn, Decl};

use std::collections::HashSet;
use super::TypecheckEnv;

impl TypecheckEnv {
    pub fn typecheck_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Import { srv_name } => todo!(),
            Decl::VarDecl { name, val } => {
                let typ = self.infer_expr(&val);
                self.var_context.insert(name.clone(), typ);
            }
            Decl::DefDecl { name, val, is_pub } => {
                let typ = self.infer_expr(&val);
                self.var_context.insert(name.clone(), typ);
            }
            Decl::TableDecl { name, records } =>  {
                let mut names = HashSet::new();
                for record in records {
                    if !names.insert(record.name.clone()) {
                        panic!("Duplicate names found in table {}", name)
                    }
                }
            }
        }
    }

    pub fn typecheck_assn(&mut self, assn: &Assn) {
        let dest_typ = self
            .var_context
            .get(&assn.dest)
            .cloned()
            .expect(&format!("cannot find {:?} in var context", assn.dest));
        let src_typ = self.infer_expr(&assn.src);

        if !self.unify(&dest_typ, &src_typ) {
            panic!(
                "cannot unify left {:?} and right {:?} in assign",
                dest_typ, src_typ
            );
        }
    }
}

// todo: assign checking
// todo: name checking
