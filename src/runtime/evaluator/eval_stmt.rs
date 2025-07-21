use std::{collections::HashSet, hash::Hash};

use crate::ast::{Assn, Decl, Entry, Expr, Insert, Record};

use super::{Evaluator, Val};

impl Evaluator {
    pub fn eval_decl(&mut self, decl: &mut Decl) -> Result<(), String> {
        match decl {
            Decl::Import { srv_name } => todo!(),
            Decl::VarDecl { name, val } => {
                self.reactive_names.insert(name.clone());

                // var should have no depend
                assert!(val.free_var(&HashSet::new(), &HashSet::new()).is_empty());

                self.eval_expr(val)?;
                self.reactive_name_to_vals.insert(name.clone(), val.clone());
            }
            Decl::DefDecl { name, val, is_pub } => {
                self.reactive_names.insert(name.clone());

                // unevaled expr of def should be stored
                self.def_name_to_exprs.insert(name.clone(), val.clone());

                // then eval def
                self.eval_expr(val)?;
                self.reactive_name_to_vals.insert(name.clone(), val.clone());
            }
            Decl::TableDecl { name, fields } => {
                self.reactive_name_to_vals.insert(name.clone(), Expr::Table {schema: fields.clone(), records:Vec::new() });
            }
        }

        Ok(())
    }

    pub fn eval_assn(&mut self, assn: &mut Assn) -> Result<(), String> {
        self.eval_expr(&mut assn.src)?;

        // since assn only appears in action,
        // their effect should not protrude to the expression level language's env
        // self.env.insert(assn.dest.clone(), assn.src.clone());
        Ok(())
    }
    pub fn eval_insert(&mut self, insert: &mut Insert) -> Result<(), String> {
        self.eval_expr(&mut insert.row)?;
        if let Expr::Rows { val: rows } = &insert.row {
            let found_table = self
                .reactive_name_to_vals
                .get_mut(&insert.table_name)
                .expect("Table not declared");
            if let Expr::Table { records, .. } = found_table {
                for row in rows {
                    let record_data = row.val.iter().map(|entry| entry.val.clone()).collect();
                    let record = Record { val: record_data };
                    records.push(record);
                }
            } else {
                panic!("Not a table");
            }
            println!("Inserted row(s), {} : {}", insert.table_name, found_table);
        }
        Ok(())
    }

    pub fn eval_assert(&mut self, expr: &mut Expr) -> Result<(), String> {
        self.eval_expr(expr)?;

        Ok(())
    }
}
