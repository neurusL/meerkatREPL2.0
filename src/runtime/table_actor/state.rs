
use crate::{ast::Expr, runtime::transaction::TxnId};


#[derive(Debug, Clone)]
pub enum TableValueState {
  Val(Expr),
  Trans(Option<Expr>, (Expr, TxnId)),   // transition state when new row is inserted
}

impl TableValueState {
  pub fn new(val: Expr) -> TableValueState {
    TableValueState::Val(val)
  }


  pub fn update(&mut self, new_val: Expr , txn_id: TxnId) {     // will only occur for insert queries
    use self::TableValueState::*;
    match self {
      TableValueState::Val(expr) => *self = Trans(Some(expr.clone()), (new_val, txn_id)),
      TableValueState::Trans( _ ,_ ) => panic!("unresolved transition state")
    }
  }

  // TODO: think about do we need trans state for tables, and consequently confirm_update since we don't use locks
  pub fn confirm_update(&mut self) -> Option<(Expr, TxnId)> {    
    if let TableValueState::Trans(_, (new_val, txn_id)) = self.clone() {
      *self = TableValueState::Val(new_val.clone());
      return Some((new_val, txn_id));
    }
    None
  }
  
  pub fn roll_back_if_relevant(&mut self, txn: &TxnId) {
    if let TableValueState::Trans(old_val, (_, write_txn)) = self {
      if txn == write_txn {
        if let Some(expr) = old_val.clone() {
          *self = TableValueState::Val(expr);
        }
        else {
          panic!("No old value to roll back to!");
        }
        
      }
    }
  }
}

impl Into<Expr> for TableValueState {
    fn into(self) -> Expr {
        match self {
            TableValueState::Val(val) => val,
            TableValueState::Trans(old_val, _) => {
                if let Some(val) = old_val {
                    val
                } else {
                    panic!("table is not initialized")
                }
            }
        }
    }
}