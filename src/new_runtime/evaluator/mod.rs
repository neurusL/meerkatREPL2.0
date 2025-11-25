use std::{collections::HashMap, fmt::Display, iter::zip, mem};

use kameo::actor::ActorRef;

use crate::{
    ast::{BinOp, Expr, UnOp},
    new_runtime::service::{ReactiveName, ReactiveRef, ServiceActor},
};

#[derive(Debug)]
pub enum EvalError {
    UnknownVariable(String),
    Other(String),
}

impl Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UnknownVariable(var) => write!(f, "unknown variable: {var}"),
            EvalError::Other(msg) => write!(f, "other error: {msg}"),
        }
    }
}

impl From<String> for EvalError {
    fn from(value: String) -> Self {
        EvalError::Other(value)
    }
}

pub struct Evaluator {
    /// Stack of local variable contexts (function parameters, let bindings, etc.)
    /// Each entry is a mapping from variable name to its value
    context_stack: Vec<HashMap<String, Expr>>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            context_stack: Vec::new(),
        }
    }

    fn push_context(&mut self, context: HashMap<String, Expr>) {
        self.context_stack.push(context);
    }

    fn pop_context(&mut self) {
        self.context_stack.pop();
    }

    fn lookup_local(&self, ident: &str) -> Option<&Expr> {
        for context in self.context_stack.iter().rev() {
            if let Some(value) = context.get(ident) {
                return Some(value);
            }
        }
        None
    }

    pub fn visit_reads(
        &mut self,
        expr: &Expr,
        service: &ActorRef<ServiceActor>,
        f: &mut impl FnMut(ReactiveRef),
    ) {
        match expr {
            Expr::Variable { ident } => {
                // Only treat as reactive if it's not a local variable
                if self.lookup_local(ident).is_none() {
                    f(ReactiveRef {
                        service: service.clone(),
                        name: ReactiveName(ident.clone()),
                    });
                }
            }
            Expr::Unop { op: _, expr } => {
                self.visit_reads(expr, service, f);
            }
            Expr::Binop {
                op: _,
                expr1,
                expr2,
            } => {
                self.visit_reads(expr1, service, f);
                self.visit_reads(expr2, service, f);
            }
            Expr::If { cond, expr1, expr2 } => {
                self.visit_reads(cond, service, f);
                self.visit_reads(expr1, service, f);
                self.visit_reads(expr2, service, f);
            }
            Expr::Func { params, body } => {
                let param_context = params
                    .iter()
                    // We use placeholder values since we only care about variable names
                    .map(|param| (param.clone(), Expr::Number { val: 0 }))
                    .collect::<HashMap<String, Expr>>();

                self.push_context(param_context);

                self.visit_reads(body, service, f);

                self.pop_context();
            }
            Expr::FuncApply { func, args } => {
                for arg in args {
                    self.visit_reads(arg, service, f);
                }
                self.visit_reads(func, service, f);
            }
            Expr::Action { assns } => {
                for assn in assns {
                    self.visit_reads(&assn.src, service, f);
                }
            }
            Expr::Number { val: _ } | Expr::Bool { val: _ } => {}
        }
    }

    pub fn visit_writes(
        &mut self,
        expr: &Expr,
        service: &ActorRef<ServiceActor>,
        f: &mut impl FnMut(ReactiveRef),
    ) {
        match expr {
            Expr::Action { assns } => {
                for assn in assns {
                    f(ReactiveRef {
                        service: service.clone(),
                        name: ReactiveName(assn.dest.clone()),
                    });
                }
            }
            _ => panic!("Cannot visit writes of {expr:?}"),
        }
    }

    pub fn eval_expr(
        &mut self,
        expr: &mut Expr,
        service: &ActorRef<ServiceActor>,
        read: &mut impl FnMut(ReactiveRef) -> Option<Expr>,
    ) -> Result<(), EvalError> {
        match expr {
            Expr::Number { val: _ } => Ok(()),
            Expr::Bool { val: _ } => Ok(()),
            Expr::Variable { ident } => {
                // First check local context stack
                if let Some(local_value) = self.lookup_local(ident) {
                    *expr = local_value.clone();
                    Ok(())
                } else {
                    // Fall back to reactive read for global variables
                    *expr = read(ReactiveRef {
                        service: service.clone(),
                        name: ReactiveName(ident.clone()),
                    })
                    .ok_or_else(|| EvalError::UnknownVariable(ident.clone()))?;
                    Ok(())
                }
            }

            Expr::Unop { op, expr: expr1 } => {
                self.eval_expr(expr1, service, read)?;
                match expr1.as_mut() {
                    Expr::Number { .. } | Expr::Bool { .. } => {
                        *expr = calc_unop(*op, expr1)?;
                        Ok(())
                    }
                    _ => Err(EvalError::Other(format!(
                        "unary operator {:?} cannot be applied to {}",
                        op, **expr1
                    ))),
                }
            }

            Expr::Binop { op, expr1, expr2 } => {
                self.eval_expr(expr1, service, read)?;
                self.eval_expr(expr2, service, read)?;
                use Expr::{Bool, Number};
                match (expr1.as_mut(), expr2.as_mut()) {
                    (Number { .. }, Number { .. }) | (Bool { .. }, Bool { .. }) => {
                        *expr = calc_binop(*op, expr1, expr2)?;
                        Ok(())
                    }
                    _ => Err(EvalError::Other(format!(
                        "binary operator {:?} cannot be applied to {} and {}",
                        op, **expr1, **expr2
                    ))),
                }
            }

            Expr::If { cond, expr1, expr2 } => {
                self.eval_expr(cond, service, read)?;
                match **cond {
                    Expr::Bool { val } => {
                        let new_expr = if val {
                            std::mem::take(expr1)
                        } else {
                            std::mem::take(expr2)
                        };
                        *expr = *new_expr;
                        self.eval_expr(expr, service, read)
                    }
                    _ => Err(EvalError::Other(format!(
                        "if condition must be a boolean, got {}",
                        **cond
                    ))),
                }
            }

            Expr::Func { params: _, body: _ } => {
                // Functions are values and don't need evaluation until applied
                Ok(())
            }

            Expr::FuncApply { func, args } => {
                self.eval_expr(func, service, read)?;

                match func.as_mut() {
                    Expr::Func { params, body } => {
                        if params.len() != args.len() {
                            Err(EvalError::Other(format!(
                                "function expects {} arguments, got {}",
                                params.len(),
                                args.len()
                            )))
                        } else {
                            // Evaluate arguments in current context
                            for arg in args.iter_mut() {
                                self.eval_expr(arg, service, read)?;
                            }

                            // Create new local context for function parameters
                            let param_context = zip(params.clone(), args.iter())
                                .map(|(param, arg)| (param, arg.clone()))
                                .collect::<HashMap<String, Expr>>();

                            // Push parameter context
                            self.push_context(param_context);

                            // Evaluate function body with parameter context
                            *expr = body.as_ref().clone();
                            let result = self.eval_expr(expr, service, read);

                            // Pop parameter context
                            self.pop_context();

                            result
                        }
                    }
                    _ => Err(EvalError::Other(format!("cannot apply non-function"))),
                }
            }

            Expr::Action { assns: _ } => {
                // Actions don't get evaluated in this context
                Ok(())
            }
        }
    }

    pub fn eval_writes(
        &mut self,
        expr: &mut Expr,
        service: &ActorRef<ServiceActor>,
        read: &mut impl FnMut(ReactiveRef) -> Option<Expr>,
        write: &mut impl FnMut(ReactiveRef, Expr),
    ) -> Result<(), EvalError> {
        match expr {
            Expr::Action { assns } => {
                let mut e = Evaluator::new();
                while !assns.is_empty() {
                    let mut assn = assns.remove(0);
                    match e.eval_expr(&mut assn.src, &service, read) {
                        Ok(()) => (),
                        Err(e) => {
                            assns.insert(0, assn);
                            return Err(e);
                        }
                    }
                    write(
                        ReactiveRef {
                            service: service.clone(),
                            name: ReactiveName(assn.dest.clone()),
                        },
                        assn.src.clone(),
                    );
                }

                Ok(())
            }
            _ => panic!("Cannot eval writes of {expr:?}"),
        }
    }
}

fn calc_unop(op: UnOp, expr: &Expr) -> Result<Expr, String> {
    if let Expr::Number { val } = expr {
        match op {
            UnOp::Neg => Ok(Expr::Number { val: -val }),
            _ => panic!(),
        }
    } else if let Expr::Bool { val } = expr {
        match op {
            UnOp::Not => Ok(Expr::Bool { val: !val }),
            _ => panic!(),
        }
    } else {
        Err(format!(
            "calculate unop expression cannot be applied to {}",
            *expr
        ))
    }
}

fn calc_binop(op: BinOp, expr1: &Expr, expr2: &Expr) -> Result<Expr, String> {
    if let (Expr::Number { val: val1 }, Expr::Number { val: val2 }) = (expr1, expr2) {
        let (val1, val2) = (*val1, *val2);
        match op {
            BinOp::Add => Ok(Expr::Number { val: val1 + val2 }),
            BinOp::Sub => Ok(Expr::Number { val: val1 - val2 }),
            BinOp::Mul => Ok(Expr::Number { val: val1 * val2 }),
            BinOp::Div => Ok(Expr::Number { val: val1 / val2 }),
            BinOp::Eq => Ok(Expr::Bool { val: val1 == val2 }),
            BinOp::Lt => Ok(Expr::Bool { val: val1 < val2 }),
            BinOp::Gt => Ok(Expr::Bool { val: val1 > val2 }),
            _ => panic!(),
        }
    } else if let (Expr::Bool { val: val1 }, Expr::Bool { val: val2 }) = (expr1, expr2) {
        let (val1, val2) = (*val1, *val2);
        match op {
            BinOp::And => Ok(Expr::Bool { val: val1 && val2 }),
            BinOp::Or => Ok(Expr::Bool { val: val1 || val2 }),
            _ => panic!(),
        }
    } else {
        Err(format!(
            "calculate binop expression cannot be applied on {} {:?} {}",
            *expr1, op, *expr2
        ))
    }
}
