use crate::ast::{Assn, DataType, Decl, Insert, Expr, Field};

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
            Decl::TableDecl { name, fields } =>  {
                let mut names = HashSet::new();
                for field in fields {
                    if !names.insert(field.name.clone()) {
                        panic!("Duplicate names found in table {}", name)
                    }
                }
                self.table_context.insert(name.clone(), fields.clone());
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
                    let infered_expr = self.infer_expr(&insert.row.val[i].val);
                    
                    if !self.unify(&infered_expr, &expected_type){
                        panic!("Data type of entry {} does not match the schema, expected {}, got {}", &insert.row.val[i].name, &expected_type, &infered_expr)
                    }
                }
                
            }
        }
        

        
        
    }

    pub fn typecheck_column_access (&self, expr: &Expr, expected_table: &str, table: &[Field]) {
        match expr {
            Expr::TableColumn { table_name, column_name } => {
                if table_name!= expected_table {
                    panic!("Field access {}.{} does not match select table {}", table_name, column_name, expected_table)
                }
                if !table.iter().any(|r| r.name == *column_name) {
                    panic!("Field {} not found in table {}", column_name, expected_table);
                }
            }
            Expr::Binop { expr1, expr2, ..} => {
                self.typecheck_column_access(expr1, expected_table, table);
                self.typecheck_column_access(expr2, expected_table, table);
            }
            Expr::Unop { expr ,..} => {
                self.typecheck_column_access(expr, expected_table, table);
            }
            Expr::If { cond, expr1, expr2 } => {
                self.typecheck_column_access(cond, expected_table, table);
                self.typecheck_column_access(expr1, expected_table, table);
                self.typecheck_column_access(expr2, expected_table, table);
            }
            Expr::FuncApply { func, args } => {
                self.typecheck_column_access(func, expected_table, table);
                for arg in args {
                    self.typecheck_column_access(arg, expected_table, table);
                }
            }
            _ => {}
        }
    }
}

// todo: assign checking
// todo: name checking