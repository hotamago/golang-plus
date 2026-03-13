use std::ops::Range;

pub type Span = Range<usize>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub package: String,
    pub imports: Vec<String>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Struct(StructDecl),
    Enum(EnumDecl),
    Function(FnDecl),
    Impl(ImplBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub derives: Vec<DeriveKind>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub derives: Vec<DeriveKind>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Vec<TypeRef>,
    pub span: Span,
}

impl EnumDecl {
    pub fn is_tagged(&self) -> bool {
        self.variants.iter().any(|v| !v.payload.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplBlock {
    pub target: String,
    pub methods: Vec<MethodDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<ParamDecl>,
    pub ret: ReturnType,
    pub body: Block,
    pub decorators: Vec<Decorator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDecl {
    pub receiver: ReceiverKind,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<ParamDecl>,
    pub ret: ReturnType,
    pub body: Block,
    pub decorators: Vec<Decorator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverKind {
    Value,
    Pointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDecl {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub raw: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    Void,
    Type(TypeRef),
    ErrorOnly,
    TypeWithError(TypeRef),
}

impl ReturnType {
    pub fn is_error_capable(&self) -> bool {
        matches!(self, Self::ErrorOnly | Self::TypeWithError(_))
    }

    pub fn value_type(&self) -> Option<&TypeRef> {
        match self {
            Self::Type(ty) | Self::TypeWithError(ty) => Some(ty),
            Self::Void | Self::ErrorOnly => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    VarDecl(VarDeclStmt),
    Return(ReturnStmt),
    Expr(ExprStmt),
    Match(MatchStmt),
    If(IfStmt),
    Raw(RawStmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarDeclStmt {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnStmt {
    pub exprs: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchStmt {
    pub value: Expr,
    pub arms: Vec<MatchArm>,
    pub resolved_enum: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchArmBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Wildcard {
        span: Span,
    },
    TypedVariant {
        enum_name: String,
        variant: String,
        bindings: Vec<String>,
        span: Span,
    },
    Variant {
        variant: String,
        bindings: Vec<String>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Self::Wildcard { span }
            | Self::TypedVariant { span, .. }
            | Self::Variant { span, .. } => span.clone(),
        }
    }

    pub fn variant_name(&self) -> Option<&str> {
        match self {
            Self::Wildcard { .. } => None,
            Self::TypedVariant { variant, .. } | Self::Variant { variant, .. } => Some(variant),
        }
    }

    pub fn bindings(&self) -> &[String] {
        match self {
            Self::Wildcard { .. } => &[],
            Self::TypedVariant { bindings, .. } | Self::Variant { bindings, .. } => bindings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_block: Block,
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStmt>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStmt {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub text: String,
    pub has_try: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decorator {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeriveKind {
    String,
}
