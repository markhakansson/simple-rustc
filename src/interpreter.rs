use crate::ast::{
    Value::{Bool, Num, Var},
    *,
};
use std::collections::HashMap;

pub type Scope = HashMap<Value, Value>; 
pub type Context = Vec<Scope>; // Context is a stack of scopes
pub type FuncContext = HashMap<String, Context>; // fn name, context

type EvalRes<T> = Result<T, EvalErr>;

#[derive(Debug, PartialEq, Eq)]
pub enum EvalErr {
    NotFound(String),
    NotImplemented,
    TypeMismatch(String),
    WrongOp(String),
    WrongType(String),
}

pub trait ContextMethods {
    fn update_var(&mut self, key: &Value, val: &Value) -> EvalRes<Value>;
    fn drop_current_scope(&mut self);
    fn get_val(&mut self, key: &Value) -> EvalRes<Value>;
    fn insert_to_current_scope(&mut self, key: &Value, val: &Value);
    fn new_scope(&mut self);
}

impl ContextMethods for Context {
    fn update_var(&mut self, key: &Value, val: &Value) -> EvalRes<Value> {
        for scope in self.iter_mut().rev() {
            match scope.get(&key) {
                Some(_) => {
                    scope.insert(key.clone(), val.clone());
                    return Ok(val.clone())
                }
                None => continue,
            }
        }

        Err(EvalErr::NotFound("Value not found in context.".to_string()))
    }

    fn drop_current_scope(&mut self) {
        self.pop();
    }

    fn get_val(&mut self, key: &Value) -> EvalRes<Value> {
        let mut val_res: EvalRes<Value> = Err(EvalErr::NotFound("Key not found in context scopes".to_string()));

        for scope in self.iter().rev() {
            match scope.get(&key) {
                Some(value) => {
                    val_res = Ok(value.clone()); 
                    break;
                },
                None => continue,
            };
        }

        val_res
    }

    fn insert_to_current_scope(&mut self, key: &Value, val: &Value) {
        let scope_opt = self.last_mut();
        match scope_opt {
            Some(scope) => scope.insert(key.clone(), val.clone()),
            None => panic!("There are no scopes in the context."),
        };
    }
    
    fn new_scope(&mut self) {
        let mut scope: Scope = HashMap::new();
        self.push(scope);
    }

}

fn eval_i32_expr(l: i32, op: Op, r: i32) -> EvalRes<Value> {
    match op {
        Op::MathOp(MathToken::Division) => Ok(Num(l / r)),
        Op::MathOp(MathToken::Multiply) => Ok(Num(l * r)),
        Op::MathOp(MathToken::Plus) => Ok(Num(l + r)),
        Op::MathOp(MathToken::Minus) => Ok(Num(l - r)),
        Op::MathOp(MathToken::Modulo) => Ok(Num(l % r)),
        Op::RelOp(RelToken::Equal) => Ok(Bool(l == r)),
        Op::RelOp(RelToken::Geq) => Ok(Bool(l > r)),
        Op::RelOp(RelToken::Leq) => Ok(Bool(l < r)),
        Op::RelOp(RelToken::Neq) => Ok(Bool(l != r)),
        _ => Err(EvalErr::WrongOp(String::from("Not an i32 operator."))),
    }
}

fn eval_bool_expr(l: bool, op: Op, r: bool) -> EvalRes<Value> {
    match op {
        Op::BoolOp(BoolToken::And) => Ok(Bool(l && r)),
        Op::BoolOp(BoolToken::Or) => Ok(Bool(l || r)),
        Op::RelOp(RelToken::Equal) => Ok(Bool(l == r)),
        Op::RelOp(RelToken::Geq) => Ok(Bool(l > r)),
        Op::RelOp(RelToken::Leq) => Ok(Bool(l < r)),
        Op::RelOp(RelToken::Neq) => Ok(Bool(l != r)),
        _ => Err(EvalErr::WrongOp(String::from("Not a boolean operator."))),
    }
}

// Evaluates whether an expression is an i32 or bool operation.
fn eval_bin_expr(l: Expr, op: Op, r: Expr, context: &mut Context) -> EvalRes<Value> {
    let l_val = eval_expr(l, context)?;
    let r_val = eval_expr(r, context)?;

    match (l_val, r_val) {
        (Num(l_val), Num(r_val)) => eval_i32_expr(l_val, op, r_val),
        (Bool(l_val), Bool(r_val)) => eval_bool_expr(l_val, op, r_val),
        _ => Err(EvalErr::TypeMismatch(String::from(
            "Can not evaluate an operation between a bool and an i32.",
        ))),
    }
}

// Evaluates a complete binomial tree to a single integer or bool.
pub fn eval_expr(e: Expr, context: &mut Context) -> EvalRes<Value> {
    match e {
        Expr::Num(num) => Ok(Num(num)),
        Expr::Bool(b) => Ok(Bool(b)),
        Expr::Var(s) => context.get_val(&Var(s)),
        Expr::BinOp(left, op, right) => eval_bin_expr(*left, op, *right, context),
        Expr::VarOp(var, op, expr) => {
            let key = Var(String::from(*var));
            let expr_val = eval_expr(*expr, context)?;

            match op {
                Op::VarOp(VarToken::Assign) => context.update_var(&key, &expr_val),
                _ => eval_var_op(&key, op, &expr_val, context),
            }
        },
        Expr::Let(var, _, expr) => assign_var(*var, *expr, context), // ignore type for now
        Expr::If(expr, block) => eval_if(*expr, block, context),
        _ => Err(EvalErr::NotImplemented),
    }
}

// Assigns value to variable. Store it in current scope.
fn assign_var(var: Expr, expr: Expr, context: &mut Context) -> EvalRes<Value> {
    let id = Var(String::from(var));
    let expr_val = eval_expr(expr, context)?;
    context.insert_to_current_scope(&id, &expr_val);
    Ok(expr_val)
}

// Evaluates variable operations such as ´a += b´ etc.
fn eval_var_op(key: &Value, op: Op, new_val: &Value, context: &mut Context) -> EvalRes<Value> {
    let old_val: i32 = i32::from(context.get_val(key)?);
    let expr_val: i32 = i32::from(new_val.clone());

    match op {
        Op::VarOp(VarToken::PlusEq) => {
            let new_val = Num(old_val + expr_val);
            context.update_var(key, &new_val)
        },
        Op::VarOp(VarToken::MinEq) => {
            let new_val = Num(old_val - expr_val);
            context.update_var(key, &new_val)
        },
        Op::VarOp(VarToken::MulEq) => {
            let new_val = Num(old_val * expr_val);
            context.update_var(key, &new_val)
        },
        _ => Err(EvalErr::WrongOp("Not a variable operator.".to_string()))
    }
}

fn eval_if(e: Expr, block: Block, context: &mut Context) -> EvalRes<Value> {
    let condition = eval_expr(e, context)?;
    let res: EvalRes<Value>;

    match condition {
        Bool(true) => {
            res = eval_block(block, context);
        }
        Bool(false) => res = Ok(Bool(false)),
        _ => {
            res = Err(EvalErr::WrongType(
                "Cannot evaluate condition. Not a boolean expression.".to_string(),
            ))
        }
    }

    res
}

// Evaluates a complete block. Returns the value from the last instruction evaluated.
pub fn eval_block(block: Block, context: &mut Context) -> EvalRes<Value> {
    context.new_scope();
    let mut res: EvalRes<Value> =
        Err(EvalErr::NotFound("No expressions found.".to_string()));

    for e in block {
        res = eval_expr(e, context);
    }
    // Should drop the scope after here
    // drop_current_scope(context);
    res
}

// TODO
/* pub fn eval_function(f: Function, args: Args, context: &mut FuncContext) {
    let mut fn_context: Context = vec![];
    context.insert(f.name, fn_context);
} */

// Main entry
//pub fn eval_program() {}