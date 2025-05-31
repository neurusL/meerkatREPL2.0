//! state of value maintained by var actor

use crate::ast::Expr;

#[derive(Debug, Clone)]
pub enum VarValueState {
    Uninit,            // uninitialized
    Val(Expr),         // stable state of a var actor value
    Trans(Option<Expr>, Expr), // when receive write request, var actor is in transition
}

impl VarValueState {
    /// when receive write (value update) request, 
    /// var actor turns into transition state
    pub fn update(&mut self, new_val: Expr) {
        use self::VarValueState::*;
        match self {
            VarValueState::Uninit => *self = Trans(None, new_val),
            VarValueState::Val(expr) => *self = Trans(Some(expr.clone()), new_val),
            VarValueState::Trans(_, _) => panic!("unrosolved transition state"),
        }
    }


    /// when receive lock release, any transition state should be confirmed 
    /// and if value is updated, return the new value
    pub fn confirm_update(&mut self) -> Option<Expr> {
        if let VarValueState::Trans(_, new_val) = self.clone() {
            *self = VarValueState::Val(new_val.clone());
            return Some(new_val)
        }
        None 
    }

    /// when receive lock abort, transition state should be rolled back 
    pub fn roll_back(&mut self) {
        if let VarValueState::Trans(old_val, _) = self {
            if let Some(val) = old_val {
                *self = VarValueState::Val(val.clone());
            } else {
                *self = VarValueState::Uninit;
            }
        }
    }

}

impl Into<Option<Expr>> for VarValueState {
    fn into(self) -> Option<Expr> {
        match self {
            VarValueState::Uninit => None,
            VarValueState::Val(val) => Some(val),
            VarValueState::Trans(_, _) => panic!("transition var value"),
        }
    }
}