use std::{collections::{HashMap, HashSet}, fmt::Display, iter::zip, mem};

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

#[derive(Debug)]
pub enum PartialEvalError {
    UnknownVariable(String),    // may not need this case for PartialEvalError, but leaving it in for now
    UnresolvedVariable(String),
    Other(String),
}

impl Display for PartialEvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartialEvalError::UnknownVariable(var) => write!(f, "unknown variable: {var}"),
            PartialEvalError::UnresolvedVariable(var) => write!(f, "unresolved variable: {var}"),
            PartialEvalError::Other(msg) => write!(f, "other error: {msg}"),
        }
    }
}

impl From<String> for PartialEvalError {
    fn from(value: String) -> Self {
        PartialEvalError::Other(value)
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

    fn pop_context(&mut self) -> Option<HashMap<String, Expr>> {
        self.context_stack.pop()
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
            Expr::Func { params, body, env:_ } => {
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
            Expr::Action { assns, env:_ } => {
                for assn in assns {
                    self.visit_reads(&assn.src, service, f);
                }
            }
            Expr::Number { val: _ } | Expr::Bool { val: _ } => {}
        }
    }

    /* Just like eval_expr but returns true or false
     * depending on whether evaluation completed */
    pub fn partial_eval(
        &mut self,
        expr: &mut Expr,
        service: &ActorRef<ServiceActor>,
        env: HashMap<String, Expr>,
        read: &mut impl FnMut(ReactiveRef) -> Option<Expr>,
    ) -> Result<bool, EvalError> {
        self.push_context(env);
        let result = match self.partial_eval_rec(expr, service, read) {
            Ok(_) => Ok(true),
            Err(PartialEvalError::UnresolvedVariable(_)) => Ok(false),
            Err(PartialEvalError::UnknownVariable(msg)) => Err(EvalError::UnknownVariable(msg)),
            Err(PartialEvalError::Other(msg)) => Err(EvalError::Other(msg)),
        };
        self.pop_context();
        result
    }

    pub fn partial_eval_rec(
        &mut self,
        expr: &mut Expr,
        service: &ActorRef<ServiceActor>,
        read: &mut impl FnMut(ReactiveRef) -> Option<Expr>,
    ) -> Result<(), PartialEvalError> {
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
                    .ok_or_else(|| PartialEvalError::UnresolvedVariable(ident.clone()))?;
                    Ok(())
                }
            }

            Expr::Unop { op, expr: expr1 } => {
                self.partial_eval_rec(expr1, service, read)?;
                match expr1.as_mut() {
                    Expr::Number { .. } | Expr::Bool { .. } => {
                        *expr = calc_unop(*op, expr1)?;
                        Ok(())
                    }
                    _ => Err(PartialEvalError::Other(format!(
                        "unary operator {:?} cannot be applied to {}",
                        op, **expr1
                    ))),
                }
            }

            Expr::Binop { op, expr1, expr2 } => {
                self.partial_eval_rec(expr1, service, read)?;
                self.partial_eval_rec(expr2, service, read)?;
                use Expr::{Bool, Number};
                match (expr1.as_mut(), expr2.as_mut()) {
                    (Number { .. }, Number { .. }) | (Bool { .. }, Bool { .. }) => {
                        *expr = calc_binop(*op, expr1, expr2)?;
                        Ok(())
                    }
                    _ => Err(PartialEvalError::Other(format!(
                        "binary operator {:?} cannot be applied to {} and {}",
                        op, **expr1, **expr2
                    ))),
                }
            }

            Expr::If { cond, expr1, expr2 } => {
                self.partial_eval_rec(cond, service, read)?;
                match **cond {
                    Expr::Bool { val } => {
                        let new_expr = if val {
                            mem::take(expr1)
                        } else {
                            mem::take(expr2)
                        };
                        *expr = *new_expr;
                        self.partial_eval_rec(expr, service, read)
                    }
                    _ => Err(PartialEvalError::Other(format!(
                        "if condition must be a boolean, got {}",
                        **cond
                    ))),
                }
            }

            Expr::Func { params, body, env } => {
                match env {
                    Some(_) => Ok(()), // already a closure, nothing to do
                    None => {
                        // it's not a closure, so create one
                        let new_env = match self.context_stack.last() {
                            Some(context) => context.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                            None => Vec::new(),
                        };
                        *expr = Expr::Func { params: params.clone(), body: body.clone(), env: Some(new_env), };
                        Ok(())
                    },
                }
            }

            Expr::FuncApply { func, args } => {
                self.partial_eval_rec(func, service, read)?;

                match func.as_mut() {
                    Expr::Func { params, body, env } => {
                        if params.len() != args.len() {
                            Err(PartialEvalError::Other(format!(
                                "function expects {} arguments, got {}",
                                params.len(),
                                args.len()
                            )))
                        } else {
                            // Evaluate arguments in current context
                            for arg in args.iter_mut() {
                                self.partial_eval_rec(arg, service, read)?;
                            }

                            // Create new local context for function parameters
                            let mut param_context = zip(params.clone(), args.iter())
                                .map(|(param, arg)| (param, arg.clone()))
                                .collect::<HashMap<String, Expr>>();

                            // Include variables from closure if they don't have the same names
                            match env {
                                Some(env_bindings) => {
                                    for binding in env_bindings.into_iter() {
                                        // add binding to param_context only if not already present
                                        if !param_context.contains_key(&binding.0) {
                                            param_context.insert(binding.0.clone(), binding.1.clone());
                                        }
                                    }
                                }
                                None => (),
                            }

                            // Push parameter context
                            self.push_context(param_context);

                            // Evaluate function body with parameter context
                            *expr = body.as_ref().clone();
                            let result = self.partial_eval_rec(expr, service, read);

                            match result {
                                Err(PartialEvalError::UnresolvedVariable(_)) => {
                                    /* We got to an unresolved variable when evaluating the function
                                     * so we need to preserve variable bindings.
                                     * Therefore, we wrap the expression inside a let.
                                     * For now, we don't have a let expression, so we use a function.
                                     * 
                                     * Either way, we pop the parameter context.
                                     */

                                    // small hack to get the expression out of expr so we can wrap it without a copy
                                    let mut temp_expr = Expr::Number { val: 0 };
                                    std::mem::swap(expr, &mut temp_expr);

                                    // create new environment from current context stack
                                    let new_env = match self.context_stack.last() {
                                        Some(context) => context.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                                        None => Vec::new(),
                                    };

                                    // TODO: add let to the core syntax and replace this code with a let
                                    *expr = Expr::FuncApply { func: Box::new(Expr::Func { params: Vec::new(), body: Box::new(temp_expr), env: Some(new_env) }), args: Vec::new() }

                                },
                                _ => { self.pop_context(); }
                            }

                            result
                        }
                    }
                    _ => Err(PartialEvalError::Other(format!("cannot apply non-function"))),
                }
            }

            Expr::Action { assns, env } => {
                match env {
                    Some(_) => Ok(()), // already a closure, nothing to do
                    None => {
                        // it's not a closure, so create one
                        let new_env = match self.context_stack.last() {
                            Some(context) => context.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                            None => Vec::new(),
                        };
                        *expr = Expr::Action { assns: assns.clone(), env: Some(new_env), };
                        Ok(())
                    },
                }
            }
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
                            mem::take(expr1)
                        } else {
                            mem::take(expr2)
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

            Expr::Func { params: _, body: _, env: _ } => {
                // Functions are values and don't need evaluation until applied
                Ok(())
            }

            Expr::FuncApply { func, args } => {
                self.eval_expr(func, service, read)?;

                match func.as_mut() {
                    Expr::Func { params, body, env:_ } => {
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

            Expr::Action { assns: _, env: _ } => {
                // Actions don't get evaluated in this context
                Ok(())
            }
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
