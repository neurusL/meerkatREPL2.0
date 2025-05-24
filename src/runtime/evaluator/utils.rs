use std::collections::HashMap;

use crate::ast::Expr;

pub fn subst(expr: &mut Expr, var_to_expr: &HashMap<String, Expr>) {
    match expr {
        Expr::Number { val } => {},
        Expr::Bool { val } => {},
        Expr::Variable { ident } => {
            if let Some(e) = var_to_expr.get(ident) {
                *expr = e.clone();
            }
        },
        Expr::Unop { op, expr } => {
            subst(expr, var_to_expr);
        },
        Expr::Binop { op, expr1, expr2 } => {
            subst(expr1, var_to_expr);
            subst(expr2, var_to_expr);
        },
        Expr::If { cond, expr1, expr2 } => {
            subst(cond, var_to_expr);
            subst(expr1, var_to_expr);
            subst(expr2, var_to_expr);
        },
        Expr::Func { params, body } => {
            todo!()
        },
        Expr::FuncApply { func, args } => {
            subst(func, var_to_expr);
            for arg in args {
                subst(arg, var_to_expr);
            }
        },
        Expr::Action { assns } => {
            for assn in assns {
                subst(&mut assn.expr, var_to_expr);
            }
        }
    }
}