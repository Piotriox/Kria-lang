use crate::bytecode::*;
use indexmap::IndexMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ArrayData {
    pub mutable: bool,
    pub elements: RefCell<Vec<Value>>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(i64),
    String(Arc<str>),
    Boolean(bool),
    Null,
    Function {
        bytecode_offset: usize,
        num_params: u32,
        upvalues: Vec<Value>,
    },
    Array(Rc<ArrayData>),
    Object(Rc<ObjectData>),
}

#[derive(Debug, Clone)]
pub struct ObjectData {
    pub fields: RefCell<IndexMap<String, Value>>,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (
                Value::Function {
                    bytecode_offset: a_off,
                    num_params: a_params,
                    upvalues: a_up,
                },
                Value::Function {
                    bytecode_offset: b_off,
                    num_params: b_params,
                    upvalues: b_up,
                },
            ) => a_off == b_off && a_params == b_params && a_up == b_up,
            (Value::Array(a), Value::Array(b)) => {
                let a_elems = a.elements.borrow();
                let b_elems = b.elements.borrow();
                a_elems.len() == b_elems.len()
                    && a_elems
                        .iter()
                        .zip(b_elems.iter())
                        .all(|(x, y)| x == y)
            }
            (Value::Object(a), Value::Object(b)) => objects_equal(a, b),
            _ => false,
        }
    }
}

fn objects_equal(a: &Rc<ObjectData>, b: &Rc<ObjectData>) -> bool {
    let a_fields = a.fields.borrow();
    let b_fields = b.fields.borrow();
    if a_fields.len() != b_fields.len() {
        return false;
    }
    a_fields
        .iter()
        .all(|(k, v)| b_fields.get(k).map(|bv| v == bv).unwrap_or(false))
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Function { bytecode_offset, num_params, upvalues } => {
                write!(
                    f,
                    "<function at {:x}({} params, {} upvalues)>",
                    bytecode_offset,
                    num_params,
                    upvalues.len()
                )
            }
            Value::Array(arr) => {
                let elems = arr.elements.borrow();
                if arr.mutable {
                    write!(f, "[")?;
                } else {
                    write!(f, "#[")?;
                }
                for (i, v) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                let fields = obj.fields.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

fn new_object() -> Value {
    Value::Object(Rc::new(ObjectData {
        fields: RefCell::new(IndexMap::new()),
    }))
}

fn value_to_string_key(key: Value) -> Result<String, String> {
    match key {
        Value::String(s) => Ok(s.to_string()),
        other => Err(format!("Object key must be a string, found {:?}", other)),
    }
}

fn object_get(obj: &Value, key: &str) -> Value {
    match obj {
        Value::Null => Value::Null,
        Value::Object(o) => o
            .fields
            .borrow()
            .get(key)
            .cloned()
            .unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn object_set(obj: &Value, key: &str, value: Value) -> Result<(), String> {
    match obj {
        Value::Object(o) => {
            o.fields.borrow_mut().insert(key.to_string(), value);
            Ok(())
        }
        Value::Null => Err("Cannot set property on null".to_string()),
        other => Err(format!("Expected object, found {:?}", other)),
    }
}

fn object_get_or_create(obj: &Value, key: &str) -> Result<Value, String> {
    match obj {
        Value::Object(o) => {
            let mut fields = o.fields.borrow_mut();
            let needs_new = match fields.get(key) {
                None => true,
                Some(Value::Null) => true,
                Some(Value::Object(_)) => false,
                Some(_) => {
                    return Err(format!(
                        "Cannot traverse through non-object property '{}'",
                        key
                    ));
                }
            };
            if needs_new {
                fields.insert(key.to_string(), new_object());
            }
            Ok(fields.get(key).unwrap().clone())
        }
        Value::Null => Err("Cannot set nested property on null".to_string()),
        other => Err(format!("Expected object, found {:?}", other)),
    }
}

fn object_delete(obj: &Value, key: &str) -> Result<(), String> {
    match obj {
        Value::Object(o) => {
            o.fields.borrow_mut().shift_remove(key);
            Ok(())
        }
        Value::Null => Ok(()),
        other => Err(format!("Expected object, found {:?}", other)),
    }
}

fn index_to_usize(index: Value) -> Result<usize, String> {
    match index {
        Value::Number(n) if n >= 0 => Ok(n as usize),
        Value::Number(_) => Err("Array index must be non-negative".to_string()),
        other => Err(format!("Array index must be a number, found {:?}", other)),
    }
}

fn expect_array(val: Value) -> Result<Rc<ArrayData>, String> {
    match val {
        Value::Array(arr) => Ok(arr),
        other => Err(format!("Expected array, found {:?}", other)),
    }
}

pub struct CallFrame {
    return_addr: usize,
    locals_start: usize,  // Index where local variables start in stack
    upvalues: Vec<Value>,
}

pub struct VM {
    stack: Vec<Value>,
    globals: Vec<Value>,
    call_stack: Vec<CallFrame>,
    bytecode: Vec<u8>,  // Keep reference to bytecode for function calls
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::with_capacity(256),
            globals: Vec::new(),
            call_stack: Vec::new(),
            bytecode: Vec::new(),
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
        let code_len = code.len();

        // Keep a copy of bytecode for function calls
        self.bytecode = code.clone();

        self.globals.resize(bytecode.num_globals, Value::Null);
        let globals_len = self.globals.len();

        let mut ip: usize = 0;

        while ip < code_len {
            let opcode = unsafe { *code.get_unchecked(ip) };
            ip += 1;

            match opcode {
                OP_RETURN => {
                    if let Some(frame) = self.call_stack.pop() {
                        // Pop return value from stack
                        let return_value = self.pop()?;
                        // Restore previous frame
                        ip = frame.return_addr;
                        // Push return value back onto stack
                        self.stack.push(return_value);
                    } else {
                        // Top-level return, just exit
                        break;
                    }
                }
                OP_LOAD_LOCAL => {
                    let offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    
                    if let Some(frame) = self.call_stack.last() {
                        let idx = frame.locals_start + offset;
                        if idx < self.stack.len() {
                            self.stack.push(self.stack[idx].clone());
                        } else {
                            return Err("Local variable out of bounds".to_string());
                        }
                    } else {
                        return Err("Load local outside function context".to_string());
                    }
                }
                OP_STORE_LOCAL => {
                    let offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let val = self.pop()?;
                    
                    if let Some(frame) = self.call_stack.last() {
                        let idx = frame.locals_start + offset;
                        if idx < self.stack.len() {
                            self.stack[idx] = val;
                        } else {
                            return Err("Local variable out of bounds".to_string());
                        }
                    } else {
                        return Err("Store local outside function context".to_string());
                    }
                }
                OP_MAKE_CLOSURE => {
                    let func_offset = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let num_params = Bytecode::read_u32(code, ip);
                    ip += 4;
                    let num_upvalues = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;

                    let mut upvalues = Vec::with_capacity(num_upvalues);
                    let parent_frame = self.call_stack.last();

                    for _ in 0..num_upvalues {
                        let kind = code[ip];
                        ip += 1;
                        let index = Bytecode::read_u32(code, ip) as usize;
                        ip += 4;

                        let value = match kind {
                            CAPTURE_LOCAL => {
                                let frame = parent_frame
                                    .ok_or_else(|| "Closure capture requires active call frame".to_string())?;
                                let idx = frame.locals_start + index;
                                if idx >= self.stack.len() {
                                    return Err("Closure local capture out of bounds".to_string());
                                }
                                self.stack[idx].clone()
                            }
                            CAPTURE_UPVALUE => {
                                let frame = parent_frame
                                    .ok_or_else(|| "Closure capture requires active call frame".to_string())?;
                                frame.upvalues.get(index)
                                    .cloned()
                                    .ok_or_else(|| "Closure upvalue capture out of bounds".to_string())?
                            }
                            _ => return Err(format!("Unknown capture kind: {}", kind)),
                        };
                        upvalues.push(value);
                    }

                    self.stack.push(Value::Function {
                        bytecode_offset: func_offset,
                        num_params,
                        upvalues,
                    });
                }
                OP_CALL_FUNCTION => {
                    let num_args = Bytecode::read_u32(code, ip) as u32;
                    ip += 4;
                    
                    let func = self.pop()?;
                    
                    match func {
                        Value::Function { bytecode_offset, num_params, upvalues } => {
                            if num_params != num_args {
                                return Err(format!(
                                    "Function expects {} arguments, got {}",
                                    num_params, num_args
                                ));
                            }
                            
                            let locals_start = self.stack.len() - num_args as usize;
                            let frame = CallFrame {
                                return_addr: ip,
                                locals_start,
                                upvalues,
                            };
                            
                            self.call_stack.push(frame);
                            ip = bytecode_offset;
                        }
                        _ => return Err("Attempted to call non-function value".to_string()),
                    }
                }
                OP_LOAD_UPVALUE => {
                    let index = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;

                    if let Some(frame) = self.call_stack.last() {
                        let val = frame.upvalues.get(index)
                            .cloned()
                            .ok_or_else(|| "Upvalue index out of bounds".to_string())?;
                        self.stack.push(val);
                    } else {
                        return Err("Load upvalue outside function context".to_string());
                    }
                }
                OP_STORE_UPVALUE => {
                    let index = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let val = self.pop()?;

                    if let Some(frame) = self.call_stack.last_mut() {
                        if index < frame.upvalues.len() {
                            frame.upvalues[index] = val;
                        } else {
                            return Err("Upvalue index out of bounds".to_string());
                        }
                    } else {
                        return Err("Store upvalue outside function context".to_string());
                    }
                }
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
                OP_BUILD_ARRAY => {
                    let count = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let mutable = code[ip] != 0;
                    ip += 1;

                    let mut elements = Vec::with_capacity(count);
                    for _ in 0..count {
                        elements.push(self.pop()?);
                    }
                    elements.reverse();

                    self.stack.push(Value::Array(Rc::new(ArrayData {
                        mutable,
                        elements: RefCell::new(elements),
                    })));
                }
                OP_INDEX_GET => {
                    let index_val = self.pop()?;
                    let container = self.pop()?;
                    match container {
                        Value::Array(arr) => {
                            let idx = index_to_usize(index_val)?;
                            let elems = arr.elements.borrow();
                            if idx >= elems.len() {
                                return Err(format!("Array index {} out of bounds", idx));
                            }
                            self.stack.push(elems[idx].clone());
                        }
                        Value::Object(_) | Value::Null => {
                            let key = value_to_string_key(index_val)?;
                            self.stack.push(object_get(&container, &key));
                        }
                        other => {
                            return Err(format!("Cannot index into {:?}", other));
                        }
                    }
                }
                OP_INDEX_SET => {
                    let value = self.pop()?;
                    let index_val = self.pop()?;
                    let container = self.pop()?;
                    match container {
                        Value::Array(arr) => {
                            if !arr.mutable {
                                return Err("Cannot modify immutable array".to_string());
                            }
                            let idx = index_to_usize(index_val)?;
                            let mut elems = arr.elements.borrow_mut();
                            if idx >= elems.len() {
                                return Err(format!("Array index {} out of bounds", idx));
                            }
                            elems[idx] = value;
                        }
                        Value::Object(_) => {
                            let key = value_to_string_key(index_val)?;
                            object_set(&container, &key, value)?;
                        }
                        Value::Null => {
                            return Err("Cannot set property on null".to_string());
                        }
                        other => {
                            return Err(format!("Cannot index into {:?}", other));
                        }
                    }
                }
                OP_ARRAY_LEN => {
                    let array_val = self.pop()?;
                    let arr = expect_array(array_val)?;
                    let len = arr.elements.borrow().len() as i64;
                    self.stack.push(Value::Number(len));
                }
                OP_ARRAY_PUSH => {
                    let value = self.pop()?;
                    let array_val = self.pop()?;
                    let arr = expect_array(array_val)?;
                    if !arr.mutable {
                        return Err("Cannot push to immutable array".to_string());
                    }
                    arr.elements.borrow_mut().push(value);
                }
                OP_ARRAY_POP => {
                    let array_val = self.pop()?;
                    let arr = expect_array(array_val)?;
                    if !arr.mutable {
                        return Err("Cannot pop from immutable array".to_string());
                    }
                    let mut elems = arr.elements.borrow_mut();
                    if elems.is_empty() {
                        return Err("Cannot pop from empty array".to_string());
                    }
                    self.stack.push(elems.pop().unwrap());
                }
                OP_BUILD_OBJECT => {
                    let count = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let mut map = IndexMap::new();
                    let mut keys = Vec::with_capacity(count);
                    for _ in 0..count {
                        let key_idx = Bytecode::read_u32(code, ip) as usize;
                        ip += 4;
                        keys.push(key_idx);
                    }
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(self.pop()?);
                    }
                    values.reverse();
                    for (i, key_idx) in keys.iter().enumerate() {
                        let key = match &constants[*key_idx] {
                            Value::String(s) => s.to_string(),
                            _ => return Err("Object key must be a string constant".to_string()),
                        };
                        map.insert(key, values[i].clone());
                    }
                    self.stack.push(Value::Object(Rc::new(ObjectData {
                        fields: RefCell::new(map),
                    })));
                }
                OP_OBJECT_GET_CONST => {
                    let key_idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let key = match &constants[key_idx] {
                        Value::String(s) => s.as_ref(),
                        _ => return Err("Object key must be a string constant".to_string()),
                    };
                    let obj = self.pop()?;
                    self.stack.push(object_get(&obj, key));
                }
                OP_OBJECT_GET => {
                    let key_val = self.pop()?;
                    let obj = self.pop()?;
                    let key = value_to_string_key(key_val)?;
                    self.stack.push(object_get(&obj, &key));
                }
                OP_OBJECT_SET => {
                    let value = self.pop()?;
                    let key_val = self.pop()?;
                    let obj = self.pop()?;
                    let key = value_to_string_key(key_val)?;
                    object_set(&obj, &key, value)?;
                }
                OP_OBJECT_DELETE => {
                    let key_val = self.pop()?;
                    let obj = self.pop()?;
                    let key = value_to_string_key(key_val)?;
                    object_delete(&obj, &key)?;
                }
                OP_OBJECT_GET_OR_CREATE_CONST => {
                    let key_idx = Bytecode::read_u32(code, ip) as usize;
                    ip += 4;
                    let key = match &constants[key_idx] {
                        Value::String(s) => s.as_ref(),
                        _ => return Err("Object key must be a string constant".to_string()),
                    };
                    let obj = self.pop()?;
                    match obj {
                        Value::Object(_) => {
                            self.stack.push(object_get_or_create(&obj, key)?);
                        }
                        Value::Null => {
                            return Err("Cannot set nested property on null".to_string());
                        }
                        other => {
                            return Err(format!("Expected object, found {:?}", other));
                        }
                    }
                }
                OP_IS_ARRAY => {
                    let val = self.pop()?;
                    self.stack.push(Value::Boolean(matches!(val, Value::Array(_))));
                }
                OP_IS_OBJECT => {
                    let val = self.pop()?;
                    self.stack.push(Value::Boolean(matches!(val, Value::Object(_))));
                }
                OP_MEMBER_LENGTH => {
                    let val = self.pop()?;
                    match val {
                        Value::Array(arr) => {
                            let len = arr.elements.borrow().len() as i64;
                            self.stack.push(Value::Number(len));
                        }
                        _ => self.stack.push(Value::Null),
                    }
                }
                OP_OBJECT_GET_OR_CREATE => {
                    let key_val = self.pop()?;
                    let obj = self.pop()?;
                    let key = value_to_string_key(key_val)?;
                    match obj {
                        Value::Object(_) => {
                            self.stack.push(object_get_or_create(&obj, &key)?);
                        }
                        Value::Null => {
                            return Err("Cannot set nested property on null".to_string());
                        }
                        other => {
                            return Err(format!("Expected object, found {:?}", other));
                        }
                    }
                }
                OP_OBJECT_KEYS => {
                    let obj = self.pop()?;
                    match obj {
                        Value::Object(o) => {
                            let fields = o.fields.borrow();
                            let keys: Vec<Value> = fields
                                .keys()
                                .map(|k| Value::String(Arc::from(k.as_str())))
                                .collect();
                            self.stack.push(Value::Array(Rc::new(ArrayData {
                                mutable: true,
                                elements: RefCell::new(keys),
                            })));
                        }
                        Value::Null => {
                            self.stack.push(Value::Array(Rc::new(ArrayData {
                                mutable: true,
                                elements: RefCell::new(Vec::new()),
                            })));
                        }
                        other => {
                            return Err(format!("Expected object for keys(), found {:?}", other));
                        }
                    }
                }
                OP_INPUT => {
                    // Read type mask
                    let type_mask = code[ip];
                    ip += 1;
                    
                    // Pop prompt from stack
                    let prompt = self.pop()?;
                    let prompt_str = match prompt {
                        Value::String(s) => s.to_string(),
                        _ => return Err("Input prompt must be a string".to_string()),
                    };
                    
                    // Print prompt
                    print!("{}", prompt_str);
                    use std::io::{self, Write};
                    let _ = io::stdout().flush();
                    
                    // Read input with retry loop
                    let has_str = (type_mask & 0x01) != 0;
                    let has_int = (type_mask & 0x02) != 0;
                    let has_float = (type_mask & 0x04) != 0;
                    
                    let value = loop {
                        let mut buffer = String::new();
                        std::io::stdin().read_line(&mut buffer)
                            .map_err(|e| format!("Input error: {}", e))?;
                        let trimmed = buffer.trim().to_string();

                        // Try integer first if permitted
                        if has_int {
                            if let Ok(num) = trimmed.parse::<i64>() {
                                break Value::Number(num);
                            }
                        }
                        
                        // Try float if permitted (and not already parsed as int)
                        if has_float && trimmed.parse::<i64>().is_err() {
                            if let Ok(num) = trimmed.parse::<f64>() {
                                break Value::Number(num as i64);
                            }
                        }
                        
                        // Try string if permitted and not numeric
                        if has_str {
                            if trimmed.parse::<i64>().is_err() && trimmed.parse::<f64>().is_err() {
                                // Pure string (non-numeric)
                                break Value::String(Arc::from(trimmed));
                            }
                        }
                        
                        // Invalid input, print error and retry
                        eprint!("Invalid input. Expected: ");
                        let mut expected = Vec::new();
                        if has_str { expected.push("string"); }
                        if has_int { expected.push("integer"); }
                        if has_float { expected.push("float"); }
                        eprintln!("{}", expected.join(" or "));
                        eprint!("Try again: ");
                        let _ = io::stderr().flush();
                    };
                    
                    self.stack.push(value);
                }
                _ => return Err(format!("Unknown opcode: {}", opcode)),
            }
        }

        Ok(())
    }
}
