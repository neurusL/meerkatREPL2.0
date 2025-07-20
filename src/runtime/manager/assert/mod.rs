use crate::{
    ast::Expr,
    runtime::{manager::Manager, message::CmdMsg, transaction::TxnPred, TestId},
};

impl Manager {
    pub async fn new_test(
        &mut self,
        name: String,
        bool_expr: Expr,
        test_id: TestId,
        preds: Vec<TxnPred>,
    ) {
        let mgr_ref = self.address.clone().unwrap();
        let actor_ref = self
            .alloc_def_actor(
                &format!("{}_assert_{}_${}", name, bool_expr, test_id),
                bool_expr,
                Some((test_id, mgr_ref, preds)),
            )
            .await
            .unwrap();

        self.test_mgrs.insert(test_id, actor_ref);
    }

    pub async fn on_test_finish(&mut self, test_id: TestId, result: bool) {
        // send AssertSucceeded back to developer channel
        self.from_developer
            .send(CmdMsg::AssertCompleted { test_id, result })
            .await
            .unwrap();

        // deallocate actor
        let actor_ref = self
            .test_mgrs
            .remove(&test_id)
            .expect(&format!("Test {} not found", test_id));

        // todo!(): remove subscriptions
        // let _ = actor_ref.stop_gracefully().await;
    }
}
