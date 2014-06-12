use std::fmt;
use collections::HashMap;
use std::iter::range_step;
use interner::{intern, InternedStr};
use std::vec::FromVec;
pub use std::default::Default;
pub use lexer::{Location, Located};

#[deriving(Clone)]
pub struct Module<Ident = InternedStr> {
    pub name : Ident,
    pub imports: ~[Import],
    pub bindings : ~[Binding<Ident>],
    pub typeDeclarations : ~[TypeDeclaration],
    pub classes : ~[Class],
    pub instances : ~[Instance<Ident>],
    pub dataDefinitions : ~[DataDefinition<Ident>]
}

#[deriving(Clone)]
pub struct Import {
    pub module: InternedStr
}

#[deriving(Clone)]
pub struct Class<Ident = InternedStr> {
    pub name : Ident,
    pub variable : TypeVariable,
    pub declarations : ~[TypeDeclaration]
}

#[deriving(Clone)]
pub struct Instance<Ident = InternedStr> {
    pub bindings : ~[Binding<Ident>],
    pub constraints : ~[Constraint],
    pub typ : Type,
    pub classname : InternedStr
}

#[deriving(Clone, PartialEq)]
pub struct Binding<Ident = InternedStr> {
    pub name : Ident,
    pub arguments: ~[Pattern<Ident>],
    pub matches: Match<Ident>,
    pub typ: Qualified<Type>
}

#[deriving(PartialEq, Eq, Clone, Show)]
pub struct Constructor<Ident = InternedStr> {
    pub name : Ident,
    pub typ : Qualified<Type>,
    pub tag : int,
    pub arity : int
}

#[deriving(PartialEq, Clone)]
pub struct DataDefinition<Ident = InternedStr> {
    pub constructors : ~[Constructor<Ident>],
    pub typ : Qualified<Type>,
    pub parameters : HashMap<InternedStr, int>
}

#[deriving(Clone, PartialEq, Eq, Default)]
pub struct TypeDeclaration {
    pub typ : Qualified<Type>,
    pub name : InternedStr
}

#[deriving(Clone, Default, PartialEq, Eq, Hash)]
pub struct TypeConstructor {
    pub name : InternedStr,
    pub kind : Kind
}

pub type VarId = InternedStr;
#[deriving(Clone, PartialEq, Eq, Default)]
pub struct TypeVariable {
    pub id : InternedStr,
    pub kind : Kind,
    pub age: int
}
#[deriving(Clone, Eq, Hash)]
pub enum Type {
    TypeVariable(TypeVariable),
    TypeConstructor(TypeConstructor),
    TypeApplication(Box<Type>, Box<Type>),
    Generic(TypeVariable)
}
#[deriving(Clone, Default, Eq, PartialEq, Hash)]
pub struct Qualified<T> {
    pub constraints: ~[Constraint],
    pub value: T
}
pub fn qualified(constraints: ~[Constraint], typ: Type) -> Qualified<Type> {
    Qualified { constraints: constraints, value: typ }
}

impl Type {

    ///Creates a new type variable with the specified id
    pub fn new_var(id : VarId) -> Type {
        Type::new_var_kind(id, StarKind)
    }
    ///Creates a new type which is a type variable which takes a number of types as arguments
    ///Gives the typevariable the correct kind arity.
    pub fn new_var_args(id: VarId, types : ~[Type]) -> Type {
        Type::new_type_kind(TypeVariable(TypeVariable { id : id, kind: StarKind, age: 0 }), types)
    }
    ///Creates a new type variable with the specified kind
    pub fn new_var_kind(id : VarId, kind: Kind) -> Type {
        TypeVariable(TypeVariable { id : id, kind: kind, age: 0 })
    }
    ///Creates a new type constructor with the specified argument and kind
    pub fn new_op(name : InternedStr, types : ~[Type]) -> Type {
        Type::new_type_kind(TypeConstructor(TypeConstructor { name : name, kind: StarKind }), types)
    }
    ///Creates a new type constructor applied to the types and with a specific kind
    pub fn new_op_kind(name : InternedStr, types : ~[Type], kind: Kind) -> Type {
        let mut result = TypeConstructor(TypeConstructor { name : name, kind: kind });
        for typ in types.move_iter() {
            result = TypeApplication(box result, box typ);
        }
        result
    }
    fn new_type_kind(mut result: Type, types: ~[Type]) -> Type {
        *result.mut_kind() = Kind::new(types.len() as int + 1);
        for typ in types.move_iter() {
            result = TypeApplication(box result, box typ);
        }
        result
    }

    ///Returns a reference to the type variable or fails if it is not a variable
    pub fn var<'a>(&'a self) -> &'a TypeVariable {
        match self {
            &TypeVariable(ref var) => var,
            _ => fail!("Tried to unwrap {} as a TypeVariable", self)
        }
    }

    ///Returns a reference to the type constructor or fails if it is not a constructor
    #[allow(dead_code)]
    pub fn ctor<'a>(&'a self) -> &'a TypeConstructor {
        match self {
            &TypeConstructor(ref op) => op,
            _ => fail!("Tried to unwrap {} as a TypeConstructor", self)
        }
    }

    ///Returns a reference to the the type function or fails if it is not an application
    #[allow(dead_code)]
    pub fn appl<'a>(&'a self) -> &'a Type {
        match self {
            &TypeApplication(ref lhs, _) => { let l: &Type = *lhs; l }
            _ => fail!("Error: Tried to unwrap {} as TypeApplication", self)
        }
    }
    #[allow(dead_code)]
    ///Returns a reference to the the type argument or fails if it is not an application
    pub fn appr<'a>(&'a self) -> &'a Type {
        match self {
            &TypeApplication(_, ref rhs) => { let r: &Type = *rhs; r }
            _ => fail!("Error: Tried to unwrap TypeApplication")
        }
    }

    ///Returns the kind of the type
    ///Fails only if the type is a type application with an invalid kind
    pub fn kind<'a>(&'a self) -> &'a Kind {
        match self {
            &TypeVariable(ref v) => &v.kind,
            &TypeConstructor(ref v) => &v.kind,
            &TypeApplication(ref lhs, _) => 
                match lhs.kind() {
                    &KindFunction(_, ref k) => {
                        let kind: &Kind = *k;
                        kind
                    }
                    _ => fail!("Type application must have a kind of KindFunction, {}", self)
                },
            &Generic(ref v) => &v.kind
        }
    }
    ///Returns a mutable reference to the types kind
    pub fn mut_kind<'a>(&'a mut self) -> &'a mut Kind {
        match *self {
            TypeVariable(ref mut v) => &mut v.kind,
            TypeConstructor(ref mut v) => &mut v.kind,
            TypeApplication(ref mut lhs, _) => 
                match *lhs.mut_kind() {
                    KindFunction(_, ref mut k) => {
                        let kind: &mut Kind = *k;
                        kind
                    }
                    _ => fail!("Type application must have a kind of KindFunction")
                },
            Generic(ref mut v) => &mut v.kind
        }
    }
}

impl <S: Writer> ::std::hash::Hash<S> for TypeVariable {
    #[inline]
    fn hash(&self, state: &mut S) {
        //Only has the id since the kind should always be the same for two variables
        self.id.hash(state);
    }
}

///Constructs a string which holds the name of an n-tuple
pub fn tuple_name(n: uint) -> String {
    let mut ident = String::from_char(1, '(');
    for _ in range(1, n) {
        ident.push_char(',');
    }
    ident.push_char(')');
    ident
}
///Returns the type of an n-tuple constructor as well as the name of the tuple
pub fn tuple_type(n: uint) -> (String, Type) {
    let mut var_list = Vec::new();
    assert!(n < 26);
    for i in range(0, n) {
        let c = (('a' as u8) + i as u8) as char;
        var_list.push(Generic(Type::new_var_kind(intern(c.to_str().as_slice()), star_kind.clone()).var().clone()));
    }
    let ident = tuple_name(n);
    let mut typ = Type::new_op(intern(ident.as_slice()), FromVec::from_vec(var_list));
    for i in range_step(n as int - 1, -1, -1) {
        let c = (('a' as u8) + i as u8) as char;
        typ = function_type_(Generic(Type::new_var(intern(c.to_str().as_slice())).var().clone()), typ);
    }
    (ident, typ)
}
///Constructs a list type which holds elements of type 'typ'
pub fn list_type(typ: Type) -> Type {
    Type::new_op(intern("[]"), ~[typ])
}
///Returns the Type of the Char type
pub fn char_type() -> Type {
    Type::new_op(intern("Char"), ~[])
}
///Returns the type for the Int type
pub fn int_type() -> Type {
    Type::new_op(intern("Int"), ~[])
}
///Returns the type for the Bool type
pub fn bool_type() -> Type {
    Type::new_op(intern("Bool"), ~[])
}
///Returns the type for the Double type
pub fn double_type() -> Type {
    Type::new_op(intern("Double"), ~[])
}
///Creates a function type
pub fn function_type(arg: &Type, result: &Type) -> Type {
    function_type_(arg.clone(), result.clone())
}

///Creates a function type
pub fn function_type_(func : Type, arg : Type) -> Type {
    Type::new_op(intern("->"), ~[func, arg])
}

///Creates a IO type
pub fn io(typ: Type) -> Type {
    Type::new_op(intern("IO"), ~[typ])
}
///Returns the unit type '()'
pub fn unit() -> Type {
    Type::new_op(intern("()"), ~[])
}


#[deriving(Clone, PartialEq, Eq, Hash)]
pub struct Constraint {
    pub class : InternedStr,
    pub variables : ~[TypeVariable]
}

#[deriving(Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    KindFunction(Box<Kind>, Box<Kind>),
    StarKind
}
impl fmt::Show for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &StarKind => write!(f, "*"),
            &KindFunction(ref lhs, ref rhs) => write!(f, "({} -> {})", *lhs, *rhs)
        }
    }
}

impl Kind {
    pub fn new(v: int) -> Kind {
        let mut kind = star_kind.clone();
        for _ in range(1, v) {
            kind = KindFunction(box StarKind, box kind);
        }
        kind
    }
}

impl Default for Kind {
    fn default() -> Kind {
        StarKind
    }
}
pub static unknown_kind : Kind = StarKind;
pub static star_kind : Kind = StarKind;

impl Default for Type {
    fn default() -> Type {
        Type::new_var(intern("a"))
    }
}
impl fmt::Show for TypeVariable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl fmt::Show for TypeConstructor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl <T: fmt::Show> fmt::Show for Qualified<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} => {}", self.constraints, self.value)
    }
}

#[deriving(PartialEq, PartialOrd)]
enum Prec_ {
    Top,
    Function,
    Constructor,
}
struct Prec<'a>(Prec_, &'a Type);

///If the type is a function it returns the type of the argument and the result type,
///otherwise it returns None
fn try_get_function<'a>(typ: &'a Type) -> Option<(&'a Type, &'a Type)> {
    match *typ {
        TypeApplication(ref xx, ref result) => {
            let y: &Type = *xx;
            match *y {
                TypeApplication(ref xx, ref arg) => {
                    let x: &Type = *xx;
                    match x {
                        &TypeConstructor(ref op) if "->" == op.name.as_slice() => {
                            let a: &Type = *arg;
                            let r: &Type = *result;
                            Some((a, r))
                        }
                        _ => None
                    }
                }
                _ => None
            }
        }
        _ => None
    }
}

impl <'a> fmt::Show for Prec<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Prec(p, t) = *self;
        match *t {
            TypeVariable(ref var) => write!(f, "{}", *var),
            TypeConstructor(ref op) => write!(f, "{}", *op),
            Generic(ref var) => write!(f, "\\#{}", *var),
            TypeApplication(ref lhs, ref rhs) => {
                match try_get_function(t) {
                    Some((arg, result)) => {
                        if p >= Function {
                            write!(f, "({} -> {})", *arg, result)
                        }
                        else {
                            write!(f, "{} -> {}", Prec(Function, arg), result)
                        }
                    }
                    None => {
                        match **lhs {
                            TypeConstructor(ref op) if "[]" == op.name.as_slice() => {
                                write!(f, "[{}]", rhs)
                            }
                            _ => {
                                if p >= Constructor {
                                    write!(f, "({} {})", Prec(Function, *lhs), Prec(Constructor, *rhs))
                                }
                                else {
                                    write!(f, "{} {}", Prec(Function, *lhs), Prec(Constructor, *rhs))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl fmt::Show for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", Prec(Top, self))
    }
}
impl fmt::Show for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.class));
        for var in self.variables.iter() {
            try!(write!(f, " {}", *var));
        }
        Ok(())
    }
}
impl fmt::Show for TypeDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} :: {}", self.name, self.typ)
    }
}

fn type_eq<'a>(mapping: &mut HashMap<&'a TypeVariable, &'a TypeVariable>, lhs: &'a Type, rhs: &'a Type) -> bool {
    match (lhs, rhs) {
        (&TypeConstructor(ref l), &TypeConstructor(ref r)) => l.name == r.name,
        (&TypeVariable(ref r), &TypeVariable(ref l)) => {
            match mapping.find(&l) {
                Some(x) => return x.id == r.id,
                None => ()
            }
            mapping.insert(l, r);
            true
        }
        (&TypeApplication(ref lhs1, ref rhs1), &TypeApplication(ref lhs2, ref rhs2)) => {
            type_eq(mapping, *lhs1, *lhs2) && type_eq(mapping, *rhs1, *rhs2)
        }
        _ => false
    }
}

impl PartialEq for Type {
    ///Compares two types, treating two type variables as equal as long as they always and only appear at the same place
    ///a -> b == c -> d
    ///a -> b != c -> c
    fn eq(&self, other: &Type) -> bool {
        let mut mapping = HashMap::new();
        type_eq(&mut mapping, self, other)
    }
}


#[deriving(Clone)]
pub struct TypedExpr<Ident = InternedStr> {
    pub expr : Expr<Ident>,
    pub typ : Type,
    pub location : Location
}

impl <T: PartialEq> PartialEq for TypedExpr<T> {
    fn eq(&self, other : &TypedExpr<T>) -> bool {
        self.expr == other.expr
    }
}

impl <T: fmt::Show> fmt::Show for TypedExpr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.expr)
    }
}

impl TypedExpr {
    pub fn new<T>(expr : Expr<T>) -> TypedExpr<T> {
        TypedExpr { expr : expr, typ : Type::new_var(intern("a")), location : Location { column : -1, row : -1, absolute : -1 } }
    }
    pub fn with_location<T>(expr : Expr<T>, loc : Location) -> TypedExpr<T> {
        TypedExpr { expr : expr, typ : Type::new_var(intern("a")), location : loc }
    }
}

#[deriving(Clone, PartialEq)]
pub struct Alternative<Ident = InternedStr> {
    pub pattern : Located<Pattern<Ident>>,
    pub matches: Match<Ident>,
}

#[deriving(Clone, PartialOrd, PartialEq, Eq)]
pub enum Pattern<Ident = InternedStr> {
    NumberPattern(int),
    IdentifierPattern(Ident),
    ConstructorPattern(Ident, ~[Pattern<Ident>]),
    WildCardPattern
}

#[deriving(Clone, PartialEq)]
pub enum Match<Ident = InternedStr> {
    Guards(~[Guard<Ident>]),
    Simple(TypedExpr<Ident>)
}
#[deriving(Clone, PartialEq, Show)]
pub struct Guard<Ident = InternedStr> {
    pub predicate: TypedExpr<Ident>,
    pub expression: TypedExpr<Ident>
}

#[deriving(Clone, PartialEq)]
pub enum DoBinding<Ident = InternedStr> {
    DoLet(~[Binding<Ident>]),
    DoBind(Located<Pattern<Ident>>, TypedExpr<Ident>),
    DoExpr(TypedExpr<Ident>)
}

#[deriving(Clone, PartialEq)]
pub enum Literal {
    Integral(int),
    Fractional(f64),
    String(InternedStr),
    Char(char)
}
#[deriving(Clone, PartialEq)]
pub enum Expr<Ident = InternedStr> {
    Identifier(Ident),
    Apply(Box<TypedExpr<Ident>>, Box<TypedExpr<Ident>>),
    Literal(Literal),
    Lambda(Pattern<Ident>, Box<TypedExpr<Ident>>),
    Let(~[Binding<Ident>], Box<TypedExpr<Ident>>),
    Case(Box<TypedExpr<Ident>>, ~[Alternative<Ident>]),
    Do(~[DoBinding<Ident>], Box<TypedExpr<Ident>>),
    TypeSig(Box<TypedExpr<Ident>>, Qualified<Type>)
}
impl <T: fmt::Show> fmt::Show for Binding<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = {}", self.name, self.matches)
    }
}

impl <T: fmt::Show> fmt::Show for Expr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write_core_expr!(*self, f, _));
        match *self {
            Do(ref bindings, ref expr) => {
                try!(write!(f, "do \\{\n"));
                for bind in bindings.iter() {
                    match *bind {
                        DoLet(ref bindings) => {
                            try!(write!(f, "let \\{\n"));
                            for bind in bindings.iter() {
                                try!(write!(f, "; {} = {}\n", bind.name, bind.matches));
                            }
                            try!(write!(f, "\\}\n"));
                        }
                        DoBind(ref p, ref e) => try!(write!(f, "; {} <- {}\n", p.node, *e)),
                        DoExpr(ref e) => try!(write!(f, "; {}\n", *e))
                    }
                }
                write!(f, "{} \\}", *expr)
            }
            _ => Ok(())
        }
    }
}
impl <T: fmt::Show> fmt::Show for Pattern<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &IdentifierPattern(ref s) => write!(f, "{}", s),
            &NumberPattern(ref i) => write!(f, "{}", i),
            &ConstructorPattern(ref name, ref patterns) => {
                try!(write!(f, "({} ", name));
                for p in patterns.iter() {
                    try!(write!(f, " {}", p));
                }
                write!(f, ")")
            }
            &WildCardPattern => write!(f, "_")
        }
    }
}

impl <T: fmt::Show> fmt::Show for Alternative<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -> {}", self.pattern.node, self.matches)
    }
}
impl <T: fmt::Show> fmt::Show for Match<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Simple(ref e) => write!(f, "{}", *e),
            Guards(ref gs) => {
                for g in gs.iter() {
                    try!(write!(f, "| {} -> {}\n", g.predicate, g.expression));
                }
                Ok(())
            }
        }
    }
}
impl fmt::Show for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Integral(i) => write!(f, "{}", i),
            Fractional(v) => write!(f, "{}", v),
            String(ref s) => write!(f, "\"{}\"", *s),
            Char(c) => write!(f, "'{}'", c)
        }
    }
}

///Trait which implements the visitor pattern.
///The tree will be walked through automatically, calling the appropriate visit_ function
///If a visit_ function is overridden it will need to call the appropriate walk_function to
///recurse deeper into the AST
pub trait Visitor<Ident> {
    fn visit_expr(&mut self, expr: &TypedExpr<Ident>) {
        walk_expr(self, expr)
    }
    fn visit_alternative(&mut self, alt: &Alternative<Ident>) {
        walk_alternative(self, alt)
    }
    fn visit_pattern(&mut self, pattern: &Pattern<Ident>) {
        walk_pattern(self, pattern)
    }
    fn visit_binding(&mut self, binding: &Binding<Ident>) {
        walk_binding(self, binding);
    }
    fn visit_module(&mut self, module: &Module<Ident>) {
        walk_module(self, module);
    }
}

pub fn walk_module<Ident>(visitor: &mut Visitor<Ident>, module: &Module<Ident>) {
    for bind in module.instances.iter().flat_map(|i| i.bindings.iter()) {
        visitor.visit_binding(bind);
    }
    for bind in module.bindings.iter() {
        visitor.visit_binding(bind);
    }
}

pub fn walk_binding<Ident>(visitor: &mut Visitor<Ident>, binding: &Binding<Ident>) {
    match binding.matches {
        Simple(ref e) => visitor.visit_expr(e),
        _ => fail!()
    }
}

pub fn walk_expr<Ident>(visitor: &mut Visitor<Ident>, expr: &TypedExpr<Ident>) {
    match &expr.expr {
        &Apply(ref func, ref arg) => {
            visitor.visit_expr(*func);
            visitor.visit_expr(*arg);
        }
        &Lambda(_, ref body) => visitor.visit_expr(*body),
        &Let(ref binds, ref e) => {
            for b in binds.iter() {
                visitor.visit_binding(b);
            }
            visitor.visit_expr(*e);
        }
        &Case(ref e, ref alts) => {
            visitor.visit_expr(*e);
            for alt in alts.iter() {
                visitor.visit_alternative(alt);
            }
        }
        &Do(ref binds, ref expr) => {
            for bind in binds.iter() {
                match *bind {
                    DoLet(ref bs) => {
                        for b in bs.iter() {
                            visitor.visit_binding(b);
                        }
                    }
                    DoBind(ref pattern, ref e) => {
                        visitor.visit_pattern(&pattern.node);
                        visitor.visit_expr(e);
                    }
                    DoExpr(ref e) => visitor.visit_expr(e)
                }
            }
            visitor.visit_expr(*expr);
        }
        &TypeSig(ref expr, _) => visitor.visit_expr(*expr),
        &Literal(..) | &Identifier(..) => ()
    }
}

pub fn walk_alternative<Ident>(visitor: &mut Visitor<Ident>, alt: &Alternative<Ident>) {
    match alt.matches {
        Simple(ref e) => visitor.visit_expr(e),
        Guards(ref gs) => {
            for g in gs.iter() {
                visitor.visit_expr(&g.predicate);
                visitor.visit_expr(&g.expression);
            }
        }
    }
}

pub fn walk_pattern<Ident>(visitor: &mut Visitor<Ident>, pattern: &Pattern<Ident>) {
    match pattern {
        &ConstructorPattern(_, ref ps) => {
            for p in ps.iter() {
                visitor.visit_pattern(p);
            }
        }
        _ => ()
    }
}
struct Binds<'a, Ident> {
    vec: &'a [Binding<Ident>]
}

impl <'a, Ident: Eq> Iterator<&'a [Binding<Ident>]> for Binds<'a, Ident> {
    fn next(&mut self) -> Option<&'a [Binding<Ident>]> {
        if self.vec.len() == 0 {
            None
        }
        else {
            let end = self.vec.iter()
                .position(|bind| bind.name != self.vec[0].name)
                .unwrap_or(self.vec.len());
            let head = self.vec.slice_to(end);
            self.vec = self.vec.slice_from(end);
            Some(head)
        }
    }
}

///Returns an iterator which returns slices which contain bindings which are next
///to eachother and have the same name.
///Ex
///not True = False
///not False = True
///undefined = ...
///Produces  [[not True, not False], [undefined]]
pub fn binding_groups<'a, Ident: Eq>(bindings: &'a [Binding<Ident>]) -> Binds<'a, Ident> {
    Binds { vec: bindings }
}

