use crate::ast::{Assn, DataType, Decl, Insert};

use std::collections::HashSet;
use super::TypecheckEnv;
use crate::static_analysis::typecheck::Type;

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
                self.table_context.insert(name.clone(), records.clone());
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

    pub fn typecheck_insert(&mut self, insert: &Insert) {
        
        let found_table = self.table_context.get(&insert.table_name).cloned();

        match found_table {
            None => panic!("Table {} for insertion not found", insert.table_name),
            Some(table) =>{
                if table.len() != insert.row.val.len() {
                    panic!("Entries in the row do not match the table {} schema", insert.table_name);
                }

                for (i,j) in table.iter().enumerate() {
                    let expected_type = match j.type_ {
                        DataType::Bool => Type::Bool,    // match statements for DataType (while parsing types from table decl) and Type enum in Typecheckenv
                        DataType::Number => Type::Int,
                        DataType::String => Type::String,
                    };
                    if self.infer_expr(&insert.row.val[i].val) != expected_type {
                        panic!("Data type of entry {} does not match the schema, expected {}, got {}", &insert.row.val[i].name, expected_type, self.infer_expr(&insert.row.val[i].val))
                    }
                }
                
            }
        }
        

        
        
    }
}

// todo: assign checking
// todo: name checking
