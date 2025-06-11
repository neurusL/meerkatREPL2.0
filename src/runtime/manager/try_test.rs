//！ test infrastructure 
//!
//!  # How to use
//!  triggered when developer run @test(service_name) {
//!     do(action);
//!     assert(boolean_expr);
//!     assert(boolean_expr); // block next do(action) until evaled(true)
//!     ...
//!     do(action);
//!     do(action);
//!     ...
//! }
//! 
//!  # implementation Idea 
//!  1. each service initialize its own manager when declared 
//!  2. test instantiated by service_name, do actions on service_name's manager 
//!  3. test treat assert(boolean_expr) by internally allocate a def actor 
//!     of boolean_expr 
//!  4. test_manager will wait for bool_expr to be true before processing next 
//!     action, on the other hand, timeout means assertion failed

use crate::{
    ast::{ReplCmd, Test, Expr}, 
    runtime::{manager::Manager, transaction::Txn},
    runtime::message::Msg,
};

impl Manager {
    pub async fn try_test(&mut self, test: Test) -> Result<(), Box<dyn std::error::Error>> {
        let mut assert_cnt = 0; // for name of assert actors

        for cmd in test.commands {
            match cmd {
                ReplCmd::Do(mut action) => {
                    self.evaluator.eval_expr(&mut action)?;
                    if let Expr::Action { assns } = action {
                        let txn = Txn::new(assns);
                        self.do_action(&txn).await?;
                    } else {
                        return Err(format!("do requires action expression").into());
                    }
                }

                ReplCmd::Assert(expr) => {
                    assert_cnt += 1;

                    let actor_ref = self.alloc_def_actor(
                        &format!("{}_assert_{}", test.name, assert_cnt),
                        expr.clone()
                    ).await?;

                    while let Some(back_msg) = actor_ref.ask(Msg::UnsafeRead).await? {
                        if let Msg::UnsafeReadResult { 
                            result: Expr::Bool { val: true } } = back_msg {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}