
use crate::ast::*;
use super::{Type,TypecheckEnv};
impl TypecheckEnv {
    pub fn typecheck_test(&mut self, test: &Test) {
        for command in test.commands.iter() {
            match command {
              ReplCmd::Do(expr) => {
                let typ = self.infer_expr(expr);
                if typ!= Type::Action {    
                  panic!("do requires action expression");
                }
              }
              ReplCmd::Assert(expr) => {
                let typ = self.infer_expr(expr);
                if typ!= Type::Bool {
                  panic!("Assert statement requires bool expression");
                }
              }
            }
        }
    }
}