use crate::bytecode::*;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    String(Arc<str>),
    Boolean(bool),
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
        }
    }
}

pub struct VM {
    stack: Vec<Value>,
    globals: Vec<Value>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::with_capacity(256),
            globals: Vec::new(),
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    #[inline(always)]
    unsafe fn global_unchecked(&self, idx: usize) -> &Value {
        self.globals.get_unchecked(idx)
    }

    #[inline(always)]
    unsafe fn set_global_unchecked(&mut self, idx: usize, val: Value) {
        *self.globals.get_unchecked_mut(idx) = val;
    }

    pub fn execute(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        let code = &bytecode.code;
        let constants = &bytecode.constants;

        self.globals.resize(bytecode.num_globals, Value::Null);
        let globals_len = self.globals.len();

        let code_len = code.len();
        let mut ip: usize = 0;

        while ip < code_len {
            let opcode = unsafe { *code.get_unchecked(ip) };
            ip += 1;

            match opcode {
                OP_LOOP_INC_LESS => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let limit = Bytecode::read_i64(code, ip);
                    ip += 8;
                    let loop_start = ip - 13;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        if n < limit {
                            unsafe { self.set_global_unchecked(idx, Value::Number(n + 1)); }
                            ip = loop_start;
                        }
                    } else {
                        return Err("LoopIncLess expects number".to_string());
                    }
                }
                OP_LESS_CONST_JUMP_IF_FALSE => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let limit = Bytecode::read_i64(code, ip);
                    ip += 8;
                    let target = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        if n >= limit {
                            ip = target;
                        }
                    } else {
                        return Err("LessConstJumpIfFalse expects number".to_string());
                    }
                }
                OP_ADD_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let rhs = Bytecode::read_i64(code, ip);
                    ip += 8;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        unsafe { self.set_global_unchecked(idx, Value::Number(n + rhs)); }
                    } else {
                        return Err("AddGlobal expects number".to_string());
                    }
                }
                OP_INC_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;

                    debug_assert!(idx < globals_len);
                    let global = unsafe { self.global_unchecked(idx) };
                    if let Value::Number(n) = *global {
                        unsafe { self.set_global_unchecked(idx, Value::Number(n + 1)); }
                    } else {
                        return Err("IncGlobal expects number".to_string());
                    }
                }
                OP_LOAD_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    debug_assert!(idx < globals_len);
                    self.stack.push(unsafe { self.global_unchecked(idx).clone() });
                }
                OP_STORE_GLOBAL => {
                    let idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let val = self.pop()?;
                    debug_assert!(idx < globals_len);
                    unsafe { self.set_global_unchecked(idx, val); }
                }
                OP_JUMP => {
                    ip = Bytecode::read_u32(code, ip) as usize;
                }
                OP_JUMP_IF_FALSE => {
                    let target = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let cond = self.pop()?;
                    match cond {
                        Value::Boolean(b) => {
                            if !b {
                                ip = target;
                            }
                        }
                        _ => return Err("JumpIfFalse expects boolean".to_string()),
                    }
                }
                OP_CONSTANT => {
                    let const_idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    self.stack.push(constants[const_idx].clone());
                }
                OP_NULL => {
                    self.stack.push(Value::Null);
                }
                OP_ADD => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Number(l + r));
                        }
                        (Value::String(l), Value::String(r)) => {
                            let concat: Arc<str> = Arc::from(format!("{}{}", l, r));
                            self.stack.push(Value::String(concat));
                        }
                        (l, r) => return Err(format!(
                            "Add requires two numbers or two strings, found {:?} and {:?}", l, r)),
                    }
                }
                OP_SUBTRACT => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Number(l - r));
                        }
                        (l, r) => return Err(format!(
                            "Subtract requires two numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_MULTIPLY => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Number(l * r));
                        }
                        (l, r) => return Err(format!(
                            "Multiply requires two numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_DIVIDE => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            if r == 0 { return Err("Division by zero".to_string()); }
                            self.stack.push(Value::Number(l / r));
                        }
                        (l, r) => return Err(format!(
                            "Divide requires two numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_EQUALS => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(left == right));
                }
                OP_NOT_EQUALS => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    self.stack.push(Value::Boolean(left != right));
                }
                OP_GREATER => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l > r));
                        }
                        (l, r) => return Err(format!(
                            "Greater requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_LESS => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l < r));
                        }
                        (l, r) => return Err(format!(
                            "Less requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_GREATER_EQUAL => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l >= r));
                        }
                        (l, r) => return Err(format!(
                            "GreaterEqual requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_LESS_EQUAL => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Number(l), Value::Number(r)) => {
                            self.stack.push(Value::Boolean(l <= r));
                        }
                        (l, r) => return Err(format!(
                            "LessEqual requires numbers, found {:?} and {:?}", l, r)),
                    }
                }
                OP_AND => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Boolean(l), Value::Boolean(r)) => {
                            self.stack.push(Value::Boolean(l && r));
                        }
                        (l, r) => return Err(format!(
                            "And requires booleans, found {:?} and {:?}", l, r)),
                    }
                }
                OP_OR => {
                    let right = self.pop()?;
                    let left = self.pop()?;
                    match (left, right) {
                        (Value::Boolean(l), Value::Boolean(r)) => {
                            self.stack.push(Value::Boolean(l || r));
                        }
                        (l, r) => return Err(format!(
                            "Or requires booleans, found {:?} and {:?}", l, r)),
                    }
                }
                OP_NOT => {
                    let operand = self.pop()?;
                    match operand {
                        Value::Boolean(b) => self.stack.push(Value::Boolean(!b)),
                        other => return Err(format!("Not requires boolean, found {:?}", other)),
                    }
                }
                OP_PRINT => {
                    let val = self.pop()?;
                    println!("{}", val);
                }
                OP_POP => {
                    self.pop()?;
                }
                _ => return Err(format!("Unknown opcode: {}", opcode)),
            }
        }

        Ok(())
    }
}
