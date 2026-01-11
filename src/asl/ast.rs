//! ASL Abstract Syntax Tree definitions

use crate::asl::types::VarDefinition;

/// Complete ASL script
#[derive(Debug, Clone)]
pub struct AslScript {
    /// State definitions (memory variables)
    pub states: Vec<StateBlock>,
    /// Startup action (runs once when process attaches)
    pub startup: Option<ActionBlock>,
    /// Shutdown action (runs once when process detaches)
    pub shutdown: Option<ActionBlock>,
    /// Init action (runs once per game start)
    pub init: Option<ActionBlock>,
    /// Exit action (runs once per game exit)
    pub exit: Option<ActionBlock>,
    /// Update action (runs every tick before other actions)
    pub update: Option<ActionBlock>,
    /// Start action (returns true to start timer)
    pub start: Option<ActionBlock>,
    /// Split action (returns true to split)
    pub split: Option<ActionBlock>,
    /// Reset action (returns true to reset)
    pub reset: Option<ActionBlock>,
    /// IsLoading action (returns true when loading)
    pub is_loading: Option<ActionBlock>,
    /// GameTime action (returns game time in seconds)
    pub game_time: Option<ActionBlock>,
}

impl Default for AslScript {
    fn default() -> Self {
        Self {
            states: Vec::new(),
            startup: None,
            shutdown: None,
            init: None,
            exit: None,
            update: None,
            start: None,
            split: None,
            reset: None,
            is_loading: None,
            game_time: None,
        }
    }
}

/// State block defining memory variables
#[derive(Debug, Clone)]
pub struct StateBlock {
    /// Process name(s) to match
    pub process_names: Vec<String>,
    /// Variable definitions
    pub variables: Vec<VarDefinition>,
}

/// Action block (code to execute)
#[derive(Debug, Clone)]
pub struct ActionBlock {
    /// Statements in this block
    pub statements: Vec<Statement>,
}

/// Statement types
#[derive(Debug, Clone)]
pub enum Statement {
    /// Variable declaration: var x = expr;
    VarDecl {
        name: String,
        value: Expression,
    },
    /// Assignment: x = expr;
    Assignment {
        target: String,
        value: Expression,
    },
    /// If statement
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    /// Return statement
    Return {
        value: Option<Expression>,
    },
    /// Expression statement (for side effects)
    Expression(Expression),
}

/// Expression types
#[derive(Debug, Clone)]
pub enum Expression {
    /// Literal values
    Literal(Literal),
    /// Variable reference: current.x, old.x, or just x
    Variable {
        scope: VarScope,
        name: String,
    },
    /// Binary operation: a + b, a == b, etc.
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    /// Unary operation: !a, -a
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    /// Function call: print("hello")
    Call {
        name: String,
        args: Vec<Expression>,
    },
    /// Member access: obj.member
    Member {
        object: Box<Expression>,
        member: String,
    },
    /// Index access: arr[i]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    /// Ternary: cond ? a : b
    Ternary {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },
}

/// Literal values
#[derive(Debug, Clone)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// Variable scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarScope {
    /// Current tick value
    Current,
    /// Previous tick value
    Old,
    /// Local variable
    Local,
    /// Settings variable
    Settings,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinaryOp {
    /// Get precedence (higher = binds tighter)
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::BitOr => 3,
            BinaryOp::BitXor => 4,
            BinaryOp::BitAnd => 5,
            BinaryOp::Eq | BinaryOp::Ne => 6,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 7,
            BinaryOp::Shl | BinaryOp::Shr => 8,
            BinaryOp::Add | BinaryOp::Sub => 9,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 10,
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Logical not: !x
    Not,
    /// Numeric negation: -x
    Neg,
    /// Bitwise not: ~x
    BitNot,
}

/// Pointer path for memory reading
#[derive(Debug, Clone)]
pub struct PointerPath {
    /// Optional module name (e.g., "game.dll")
    pub module: Option<String>,
    /// Offset chain
    pub offsets: Vec<i64>,
}

impl PointerPath {
    pub fn new(offsets: Vec<i64>) -> Self {
        Self {
            module: None,
            offsets,
        }
    }

    pub fn with_module(module: String, offsets: Vec<i64>) -> Self {
        Self {
            module: Some(module),
            offsets,
        }
    }
}
