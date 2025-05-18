use crate::parser::{Strategy, Indicator, Conditions, Actions, Loop, Value, Signal};
use crate::stdlib::{StandardLibrary, DataType};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// 中間表示（Intermediate Representation）
#[derive(Debug, Clone)]
pub enum IR {
    // 基本操作
    Constant(Value),
    Variable(String),
    Load(String),
    Store(String, Box<IR>),
    
    // 算術操作
    Add(Box<IR>, Box<IR>),
    Sub(Box<IR>, Box<IR>),
    Mul(Box<IR>, Box<IR>),
    Div(Box<IR>, Box<IR>),
    Mod(Box<IR>, Box<IR>),
    
    // 比較操作
    Eq(Box<IR>, Box<IR>),
    Ne(Box<IR>, Box<IR>),
    Lt(Box<IR>, Box<IR>),
    Le(Box<IR>, Box<IR>),
    Gt(Box<IR>, Box<IR>),
    Ge(Box<IR>, Box<IR>),
    
    // 邏輯操作
    And(Box<IR>, Box<IR>),
    Or(Box<IR>, Box<IR>),
    Not(Box<IR>),
    
    // 函數調用
    Call(String, Vec<IR>),
    
    // 指標調用
    Indicator(String, HashMap<String, IR>),
    
    // 數據存取
    Access(String, Option<Box<IR>>), // e.g., close, close[0]
    
    // 控制流
    If(Box<IR>, Vec<IR>, Vec<IR>),
    While(Box<IR>, Vec<IR>),
    For(String, Box<IR>, Box<IR>, Box<IR>, Vec<IR>), // var, start, end, step, body
    ForEach(String, Box<IR>, Vec<IR>), // var, collection, body
    
    // 動作
    Action(String, HashMap<String, IR>),
    
    // 複合
    Block(Vec<IR>),
    
    // 其他
    Return(Option<Box<IR>>),
    Break,
    Continue,
    Nop,
}

/// 編譯錯誤
#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub location: Option<Location>,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.location {
            Some(loc) => write!(f, "{}:{}: {}", loc.line, loc.column, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl Error for CompileError {}

/// DSL 編譯器
pub struct Compiler {
    stdlib: StandardLibrary,
    symbols: SymbolTable,
    errors: Vec<CompileError>,
}

/// 符號表
#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub data_type: DataType,
    pub mutable: bool,
    pub value: Option<Value>,
}

impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            scopes: vec![HashMap::new()],
        }
    }
    
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    
    fn declare(&mut self, name: String, symbol: Symbol) -> Result<(), String> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name) {
                return Err(format!("Symbol '{}' already declared in this scope", name));
            }
            scope.insert(name, symbol);
            Ok(())
        } else {
            Err("No active scope".to_string())
        }
    }
    
    fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            stdlib: StandardLibrary::new(),
            symbols: SymbolTable::new(),
            errors: Vec::new(),
        }
    }
    
    /// 編譯策略
    pub fn compile(&mut self, strategy: &Strategy) -> Result<CompiledStrategy, Vec<CompileError>> {
        self.errors.clear();
        let mut instructions = Vec::new();
        
        // 編譯參數
        if let Some(params) = &strategy.parameters {
            for (name, value) in params {
                let ir = self.compile_value(value)?;
                instructions.push(IR::Store(name.clone(), Box::new(ir)));
            }
        }
        
        // 編譯指標
        if let Some(indicators) = &strategy.indicators {
            for indicator in indicators {
                let ir = self.compile_indicator(indicator)?;
                instructions.push(ir);
            }
        }
        
        // 編譯信號
        if let Some(signals) = &strategy.signals {
            for (name, signal) in signals {
                let ir = self.compile_signal(signal)?;
                instructions.push(IR::Store(format!("signal_{}", name), Box::new(ir)));
            }
        }
        
        // 編譯迴圈
        if let Some(loops) = &strategy.loops {
            for loop_def in loops {
                let ir = self.compile_loop(loop_def)?;
                instructions.push(ir);
            }
        }
        
        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }
        
        Ok(CompiledStrategy {
            name: strategy.name.clone(),
            version: strategy.version.clone(),
            instructions: IR::Block(instructions),
            symbols: self.symbols.clone(),
        })
    }
    
    /// 編譯值
    fn compile_value(&mut self, value: &Value) -> Result<IR, CompileError> {
        match value {
            Value::String(s) => Ok(IR::Constant(Value::String(s.clone()))),
            Value::Number(n) => Ok(IR::Constant(Value::Number(*n))),
            Value::Integer(i) => Ok(IR::Constant(Value::Integer(*i))),
            Value::Boolean(b) => Ok(IR::Constant(Value::Boolean(*b))),
            Value::Array(arr) => {
                let elements: Result<Vec<_>, _> = arr.iter()
                    .map(|v| self.compile_value(v))
                    .collect();
                match elements {
                    Ok(elems) => Ok(IR::Call("array".to_string(), elems)),
                    Err(e) => Err(e),
                }
            }
            Value::Object(obj) => {
                let mut fields = HashMap::new();
                for (k, v) in obj {
                    fields.insert(k.clone(), self.compile_value(v)?);
                }
                Ok(IR::Call("object".to_string(), vec![]))
            }
        }
    }
    
    /// 編譯指標
    fn compile_indicator(&mut self, indicator: &Indicator) -> Result<IR, CompileError> {
        let mut params = HashMap::new();
        
        if let Some(indicator_params) = &indicator.params {
            for (name, value) in indicator_params {
                params.insert(name.clone(), self.compile_value(value)?);
            }
        }
        
        // 驗證指標
        if let Some(def) = self.stdlib.get_indicator(&indicator.name) {
            // 檢查必需參數
            for param_def in &def.params {
                if !param_def.optional && !params.contains_key(&param_def.name) {
                    return Err(CompileError {
                        message: format!("Missing required parameter '{}' for indicator '{}'", 
                                       param_def.name, indicator.name),
                        location: None,
                    });
                }
            }
        } else {
            return Err(CompileError {
                message: format!("Unknown indicator: {}", indicator.name),
                location: None,
            });
        }
        
        Ok(IR::Indicator(indicator.name.clone(), params))
    }
    
    /// 編譯條件表達式
    fn compile_condition(&mut self, condition: &str) -> Result<IR, CompileError> {
        // 簡化的條件解析器
        // 在實際實現中，這裡需要一個完整的表達式解析器
        
        if condition.contains(" > ") {
            let parts: Vec<&str> = condition.split(" > ").collect();
            if parts.len() == 2 {
                let left = self.compile_expression(parts[0])?;
                let right = self.compile_expression(parts[1])?;
                return Ok(IR::Gt(Box::new(left), Box::new(right)));
            }
        } else if condition.contains(" < ") {
            let parts: Vec<&str> = condition.split(" < ").collect();
            if parts.len() == 2 {
                let left = self.compile_expression(parts[0])?;
                let right = self.compile_expression(parts[1])?;
                return Ok(IR::Lt(Box::new(left), Box::new(right)));
            }
        } else if condition.contains(" == ") {
            let parts: Vec<&str> = condition.split(" == ").collect();
            if parts.len() == 2 {
                let left = self.compile_expression(parts[0])?;
                let right = self.compile_expression(parts[1])?;
                return Ok(IR::Eq(Box::new(left), Box::new(right)));
            }
        }
        
        Err(CompileError {
            message: format!("Cannot parse condition: {}", condition),
            location: None,
        })
    }
    
    /// 編譯表達式
    fn compile_expression(&mut self, expr: &str) -> Result<IR, CompileError> {
        let expr = expr.trim();
        
        // 數字常量
        if let Ok(n) = expr.parse::<f64>() {
            return Ok(IR::Constant(Value::Number(n)));
        }
        
        // 字符串常量
        if expr.starts_with('"') && expr.ends_with('"') {
            let s = expr[1..expr.len()-1].to_string();
            return Ok(IR::Constant(Value::String(s)));
        }
        
        // 函數調用
        if expr.contains('(') && expr.ends_with(')') {
            let paren_pos = expr.find('(').unwrap();
            let func_name = &expr[..paren_pos];
            let args_str = &expr[paren_pos+1..expr.len()-1];
            
            // 解析參數
            let args: Vec<&str> = if args_str.is_empty() {
                vec![]
            } else {
                args_str.split(',').map(|s| s.trim()).collect()
            };
            
            let compiled_args: Result<Vec<_>, _> = args.iter()
                .map(|arg| self.compile_expression(arg))
                .collect();
                
            return Ok(IR::Call(func_name.to_string(), compiled_args?));
        }
        
        // 變量或數據存取
        if expr.contains('.') {
            let parts: Vec<&str> = expr.split('.').collect();
            if parts.len() == 2 {
                // 處理如 symbol.close 的情況
                return Ok(IR::Access(expr.to_string(), None));
            }
        }
        
        // 數組索引
        if expr.contains('[') && expr.ends_with(']') {
            let bracket_pos = expr.find('[').unwrap();
            let base = &expr[..bracket_pos];
            let index_str = &expr[bracket_pos+1..expr.len()-1];
            
            let base_ir = self.compile_expression(base)?;
            let index_ir = self.compile_expression(index_str)?;
            
            return Ok(IR::Access(base.to_string(), Some(Box::new(index_ir))));
        }
        
        // 簡單變量
        Ok(IR::Variable(expr.to_string()))
    }
    
    /// 編譯信號
    fn compile_signal(&mut self, signal: &Signal) -> Result<IR, CompileError> {
        let mut conditions = Vec::new();
        
        // 編譯 all_of 條件
        if let Some(all_of) = &signal.when.all_of {
            let all_conditions: Result<Vec<_>, _> = all_of.iter()
                .map(|cond| self.compile_condition(cond))
                .collect();
                
            match all_conditions {
                Ok(conds) => {
                    let and_condition = conds.into_iter()
                        .reduce(|acc, cond| IR::And(Box::new(acc), Box::new(cond)))
                        .unwrap_or(IR::Constant(Value::Boolean(true)));
                    conditions.push(and_condition);
                }
                Err(e) => return Err(e),
            }
        }
        
        // 編譯 any_of 條件
        if let Some(any_of) = &signal.when.any_of {
            let any_conditions: Result<Vec<_>, _> = any_of.iter()
                .map(|cond| self.compile_condition(cond))
                .collect();
                
            match any_conditions {
                Ok(conds) => {
                    let or_condition = conds.into_iter()
                        .reduce(|acc, cond| IR::Or(Box::new(acc), Box::new(cond)))
                        .unwrap_or(IR::Constant(Value::Boolean(false)));
                    conditions.push(or_condition);
                }
                Err(e) => return Err(e),
            }
        }
        
        // 組合所有條件
        let final_condition = conditions.into_iter()
            .reduce(|acc, cond| IR::And(Box::new(acc), Box::new(cond)))
            .unwrap_or(IR::Constant(Value::Boolean(true)));
        
        // 編譯動作
        let mut actions = Vec::new();
        for action in &signal.actions {
            actions.push(self.compile_action(action)?);
        }
        
        Ok(IR::If(
            Box::new(final_condition),
            actions,
            vec![]
        ))
    }
    
    /// 編譯動作
    fn compile_action(&mut self, action: &crate::parser::Action) -> Result<IR, CompileError> {
        let mut params = HashMap::new();
        
        for (name, value) in &action.params {
            params.insert(name.clone(), self.compile_value(value)?);
        }
        
        Ok(IR::Action(action.action_type.clone(), params))
    }
    
    /// 編譯迴圈
    fn compile_loop(&mut self, loop_def: &Loop) -> Result<IR, CompileError> {
        match loop_def {
            Loop::For { name, iterator, values, condition, action, body, .. } => {
                // 創建新作用域
                self.symbols.push_scope();
                
                // 聲明迭代變量
                self.symbols.declare(iterator.clone(), Symbol {
                    name: iterator.clone(),
                    data_type: DataType::Any,
                    mutable: false,
                    value: None,
                })?;
                
                // 編譯值列表
                let values_ir = self.compile_value(values)?;
                
                // 編譯循環體
                let mut loop_body = Vec::new();
                
                // 編譯條件
                if let Some(conditions) = condition {
                    for cond in conditions {
                        let cond_ir = self.compile_condition(cond)?;
                        loop_body.push(IR::If(
                            Box::new(cond_ir),
                            vec![],
                            vec![IR::Continue]
                        ));
                    }
                }
                
                // 編譯動作
                if let Some(act) = action {
                    loop_body.push(self.compile_action(act)?);
                }
                
                // 編譯嵌套body
                if let Some(inner_body) = body {
                    for inner_loop in inner_body {
                        loop_body.push(self.compile_loop(inner_loop)?);
                    }
                }
                
                self.symbols.pop_scope();
                
                Ok(IR::ForEach(
                    iterator.clone(),
                    Box::new(values_ir),
                    loop_body
                ))
            }
            
            Loop::ForRange { name, variable, from, to, step, action, body, .. } => {
                self.symbols.push_scope();
                
                self.symbols.declare(variable.clone(), Symbol {
                    name: variable.clone(),
                    data_type: DataType::Numeric,
                    mutable: false,
                    value: None,
                })?;
                
                let from_ir = IR::Constant(Value::Number(*from));
                let to_ir = IR::Constant(Value::Number(*to));
                let step_ir = IR::Constant(Value::Number(*step));
                
                let mut loop_body = Vec::new();
                
                if let Some(act) = action {
                    loop_body.push(self.compile_action(act)?);
                }
                
                if let Some(inner_body) = body {
                    for inner_loop in inner_body {
                        loop_body.push(self.compile_loop(inner_loop)?);
                    }
                }
                
                self.symbols.pop_scope();
                
                Ok(IR::For(
                    variable.clone(),
                    Box::new(from_ir),
                    Box::new(to_ir),
                    Box::new(step_ir),
                    loop_body
                ))
            }
            
            Loop::ForEach { name, collection, as_variable, when, action, body, .. } => {
                self.symbols.push_scope();
                
                self.symbols.declare(as_variable.clone(), Symbol {
                    name: as_variable.clone(),
                    data_type: DataType::Any,
                    mutable: false,
                    value: None,
                })?;
                
                let collection_ir = self.compile_expression(collection)?;
                
                let mut loop_body = Vec::new();
                
                // 編譯when條件
                if let Some(conditions) = when {
                    for cond in conditions {
                        let cond_ir = self.compile_condition(cond)?;
                        loop_body.push(IR::If(
                            Box::new(cond_ir),
                            vec![],
                            vec![IR::Continue]
                        ));
                    }
                }
                
                if let Some(act) = action {
                    loop_body.push(self.compile_action(act)?);
                }
                
                if let Some(inner_body) = body {
                    for inner_loop in inner_body {
                        loop_body.push(self.compile_loop(inner_loop)?);
                    }
                }
                
                self.symbols.pop_scope();
                
                Ok(IR::ForEach(
                    as_variable.clone(),
                    Box::new(collection_ir),
                    loop_body
                ))
            }
            
            Loop::While { name, condition, max_iterations, body, .. } => {
                self.symbols.push_scope();
                
                // 編譯條件
                let mut combined_condition = None;
                for cond in condition {
                    let cond_ir = self.compile_condition(cond)?;
                    combined_condition = match combined_condition {
                        None => Some(cond_ir),
                        Some(existing) => Some(IR::And(Box::new(existing), Box::new(cond_ir))),
                    };
                }
                
                let final_condition = combined_condition
                    .unwrap_or(IR::Constant(Value::Boolean(true)));
                
                // 編譯循環體
                let mut loop_body = Vec::new();
                for action in body {
                    loop_body.push(self.compile_action(action)?);
                }
                
                // 添加迭代計數器檢查
                if let Some(max_iter) = max_iterations {
                    // 這裡簡化處理，實際應該維護計數器
                    loop_body.push(IR::If(
                        Box::new(IR::Ge(
                            Box::new(IR::Variable("_iteration".to_string())),
                            Box::new(IR::Constant(Value::Integer(*max_iter as i64)))
                        )),
                        vec![IR::Break],
                        vec![]
                    ));
                }
                
                self.symbols.pop_scope();
                
                Ok(IR::While(
                    Box::new(final_condition),
                    loop_body
                ))
            }
        }
    }
}

/// 編譯後的策略
#[derive(Debug)]
pub struct CompiledStrategy {
    pub name: String,
    pub version: String,
    pub instructions: IR,
    pub symbols: SymbolTable,
}

impl CompiledStrategy {
    /// 優化IR
    pub fn optimize(&mut self) {
        self.instructions = self.optimize_ir(self.instructions.clone());
    }
    
    fn optimize_ir(&self, ir: IR) -> IR {
        match ir {
            // 常量折疊
            IR::Add(left, right) => {
                match (self.optimize_ir(*left), self.optimize_ir(*right)) {
                    (IR::Constant(Value::Number(a)), IR::Constant(Value::Number(b))) => {
                        IR::Constant(Value::Number(a + b))
                    }
                    (left, right) => IR::Add(Box::new(left), Box::new(right)),
                }
            }
            
            // 死代碼刪除
            IR::If(cond, then_branch, else_branch) => {
                let opt_cond = self.optimize_ir(*cond);
                match opt_cond {
                    IR::Constant(Value::Boolean(true)) => {
                        IR::Block(then_branch.into_iter()
                            .map(|ir| self.optimize_ir(ir))
                            .collect())
                    }
                    IR::Constant(Value::Boolean(false)) => {
                        IR::Block(else_branch.into_iter()
                            .map(|ir| self.optimize_ir(ir))
                            .collect())
                    }
                    _ => IR::If(
                        Box::new(opt_cond),
                        then_branch.into_iter()
                            .map(|ir| self.optimize_ir(ir))
                            .collect(),
                        else_branch.into_iter()
                            .map(|ir| self.optimize_ir(ir))
                            .collect()
                    ),
                }
            }
            
            // 遞歸優化其他節點
            IR::Block(instructions) => {
                IR::Block(instructions.into_iter()
                    .map(|ir| self.optimize_ir(ir))
                    .filter(|ir| !matches!(ir, IR::Nop))
                    .collect())
            }
            
            // 對於其他情況，返回原值
            _ => ir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::*;
    
    #[test]
    fn test_compile_simple_strategy() {
        let mut compiler = Compiler::new();
        
        let strategy = Strategy {
            name: "Test Strategy".to_string(),
            version: "1.0".to_string(),
            description: None,
            tags: None,
            universe: None,
            execution: None,
            parameters: Some(HashMap::from([
                ("lookback".to_string(), Value::Integer(20)),
                ("threshold".to_string(), Value::Number(0.02)),
            ])),
            assets: None,
            indicators: None,
            conditions: None,
            actions: None,
            risk_management: None,
            state_machine: None,
            loops: None,
            signals: None,
        };
        
        let result = compiler.compile(&strategy);
        assert!(result.is_ok());
        
        let compiled = result.unwrap();
        assert_eq!(compiled.name, "Test Strategy");
        assert_eq!(compiled.version, "1.0");
    }
    
    #[test]
    fn test_compile_indicator() {
        let mut compiler = Compiler::new();
        
        let indicator = Indicator {
            name: "ma".to_string(),
            params: Some(HashMap::from([
                ("period".to_string(), Value::Integer(20)),
                ("type".to_string(), Value::String("simple".to_string())),
            ])),
            formula: None,
            inputs: None,
            output: None,
        };
        
        let result = compiler.compile_indicator(&indicator);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_compile_expression() {
        let mut compiler = Compiler::new();
        
        // 測試數字
        let result = compiler.compile_expression("42");
        assert!(result.is_ok());
        
        // 測試字符串
        let result = compiler.compile_expression("\"hello\"");
        assert!(result.is_ok());
        
        // 測試函數調用
        let result = compiler.compile_expression("ma(20)");
        assert!(result.is_ok());
        
        // 測試數據存取
        let result = compiler.compile_expression("close[0]");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_optimize_ir() {
        let compiled = CompiledStrategy {
            name: "Test".to_string(),
            version: "1.0".to_string(),
            instructions: IR::Add(
                Box::new(IR::Constant(Value::Number(1.0))),
                Box::new(IR::Constant(Value::Number(2.0)))
            ),
            symbols: SymbolTable::new(),
        };
        
        let optimized = compiled.optimize_ir(compiled.instructions);
        
        match optimized {
            IR::Constant(Value::Number(n)) => assert_eq!(n, 3.0),
            _ => panic!("Expected constant 3.0"),
        }
    }
}