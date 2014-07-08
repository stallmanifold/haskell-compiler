use module::{DataDefinition, encode_binding_identifier};
use core::*;
use renamer::{Name, NameSupply};
use types::*;
use interner::intern;

use std::vec::FromVec;

pub fn generate_deriving(bindings: &mut Vec<Binding<Id<Name>>>, data: &DataDefinition<Name>) {
    let mut gen = DerivingGen { name_supply: NameSupply::new() };
    for deriving in data.deriving.iter() {
        match deriving.as_slice() {
            "Eq" => {
                bindings.push(gen.generate_eq(data));
            }
            "Ord" => {
                let b = gen.generate_ord(data);
                debug!("Generated Ord {} ->>\n{}", data.typ, b);
                bindings.push(b);
            }
            x => fail!("Cannot generate instance for class {}", x)
        }
    }
}

struct DerivingGen {
    name_supply: NameSupply
}
impl DerivingGen {
    fn generate_eq(&mut self, data: &DataDefinition<Name>) -> Binding<Id<Name>> {
        self.make_binop("==", data, |this, id_l, id_r| {
            let alts = this.match_same_constructors(data, &id_r, |this, l, r| this.eq_fields(l, r));
            Case(box Identifier(id_l.clone()), alts)
        })
    }

    fn eq_fields(&mut self, args_l: &[Id<Name>], args_r: &[Id<Name>]) -> Expr<Id<Name>> {
        if args_l.len() >= 1 {
            let first = bool_binop("==", Identifier(args_l[0].clone()), Identifier(args_r[0].clone()));
            args_l.iter().skip(1).zip(args_r.iter().skip(1)).fold(first, |acc, (l, r)| {
                let test = bool_binop("==", Identifier(l.clone()), Identifier(r.clone()));
                bool_binop("&&", acc, test)
            })
        }
        else {
            true_expr()
        }
    }

    fn generate_ord(&mut self, data: &DataDefinition<Name>) -> Binding<Id<Name>> {
        self.make_binop("compare", data, |this, id_l, id_r| {
            //We first compare the tags of the arguments since this would otherwise the last of the alternatives
            let when_eq = {
                let alts = this.match_same_constructors(data, &id_r, |this, l, r| this.ord_fields(l, r));
                Case(box Identifier(id_l.clone()), alts)
            };
            let cmp = compare_tags(Identifier(id_l), Identifier(id_r));
            this.eq_or_default(cmp, when_eq)
        })
    }

    fn ord_fields(&mut self, args_l: &[Id<Name>], args_r: &[Id<Name>]) -> Expr<Id<Name>> {
        let ordering = Type::new_op(intern("Ordering"), box []);
        if args_l.len() >= 1 {
            let mut iter = args_l.iter().zip(args_r.iter()).rev();
            let (x, y) = iter.next().unwrap();
            let last = binop("compare", Identifier(x.clone()), Identifier(y.clone()), ordering.clone());
            iter.fold(last, |acc, (l, r)| {
                let test = bool_binop("compare", Identifier(l.clone()), Identifier(r.clone()));
                self.eq_or_default(test, acc)
            })
        }
        else {
            Identifier(id("EQ", ordering))
        }
    }

    ///Creates a binary function binding with the name 'funcname' which is a function in an instance for 'data'
    ///This function takes two parameters of the type of 'data'
    fn make_binop(&mut self, funcname: &str, data: &DataDefinition<Name>, func: |&mut DerivingGen, Id<Name>, Id<Name>| -> Expr<Id<Name>>) -> Binding<Id<Name>> {
        let arg_l = self.name_supply.anonymous();
        let arg_r = self.name_supply.anonymous();
        let id_r = Id::new(arg_r, data.typ.value.clone(), data.typ.constraints.clone());
        let id_l = Id::new(arg_l, data.typ.value.clone(), data.typ.constraints.clone());
        let expr = func(self, id_l.clone(), id_r.clone());
        let lambda_expr = Lambda(id_l, box Lambda(id_r, box expr));//TODO types
        let data_name = extract_applied_type(&data.typ.value).ctor().name;
        let name = encode_binding_identifier(data_name, intern(funcname));
        Binding {
            name: Id::new(Name { name: name, uid: 0 }, lambda_expr.get_type().clone(), box []),
            expression: lambda_expr
        }
    }

    fn eq_or_default(&mut self, cmp: Expr<Id<Name>>, def: Expr<Id<Name>>) -> Expr<Id<Name>> {
        let match_id = Id::new(self.name_supply.anonymous(), Type::new_op(intern("Ordering"), box []), box []);
        Case(box cmp, box [
            Alternative {
                pattern: ConstructorPattern(id("EQ", Type::new_op(intern("Ordering"), box [])), box []),
                expression: def
            },
            Alternative { pattern: IdentifierPattern(match_id.clone()), expression: Identifier(match_id) }
        ])
    }

    fn match_same_constructors(&mut self, data: &DataDefinition<Name>, id_r: &Id<Name>, f: |&mut DerivingGen, &[Id<Name>], &[Id<Name>]| -> Expr<Id<Name>>) -> ~[Alternative<Id<Name>>] {
        let alts: Vec<Alternative<Id<Name>>> = data.constructors.iter().map(|constructor| {
            let args_l: ~[Id<Name>] = FromVec::<Id<Name>>::from_vec(
                ArgIterator { typ: &constructor.typ.value }
                .map(|arg| Id::new(self.name_supply.anonymous(), arg.clone(), constructor.typ.constraints.clone()))
                .collect());
            let iter = ArgIterator { typ: &constructor.typ.value };
            let args_r: ~[Id<Name>] = FromVec::<Id<Name>>::from_vec(iter
                .map(|arg| Id::new(self.name_supply.anonymous(), arg.clone(), constructor.typ.constraints.clone()))
                .collect());
            let ctor_id = Id::new(constructor.name, iter.typ.clone(), constructor.typ.constraints.clone());
            let expr = f(self, args_l, args_r);
            let pattern_r = ConstructorPattern(ctor_id.clone(), args_r);
            let inner = Case(box Identifier(id_r.clone()), box [
                Alternative { pattern: pattern_r, expression: expr },
                Alternative { 
                    pattern: WildCardPattern,
                    expression: Identifier(Id::new(Name { uid: 0, name: intern("False") }, bool_type(), box []))
                }
            ]);
            Alternative { pattern: ConstructorPattern(ctor_id, args_l), expression: inner }
        }).collect();
        FromVec::from_vec(alts)
    }
}


fn id(s: &str, typ: Type) -> Id<Name> {
    Id::new(Name {name: intern(s), uid: 0 }, typ, box [])
}

fn compare_tags(lhs: Expr<Id<Name>>, rhs: Expr<Id<Name>>) -> Expr<Id<Name>> {
    let var = Type::new_var(intern("a"));
    let typ = function_type_(var.clone(), function_type_(var.clone(), Type::new_op(intern("Ordering"), box [])));
    let id = Id::new(Name { name: intern("#compare_tags"), uid: 0 }, typ, box []);
    Apply(box Apply(box Identifier(id), box lhs), box rhs)
}

fn bool_binop(op: &str, lhs: Expr<Id<Name>>, rhs: Expr<Id<Name>>) -> Expr<Id<Name>> {
    binop(op, lhs, rhs, bool_type())
}
fn binop(op: &str, lhs: Expr<Id<Name>>, rhs: Expr<Id<Name>>, return_type: Type) -> Expr<Id<Name>> {
    let typ = function_type_(lhs.get_type().clone(), function_type_(rhs.get_type().clone(), return_type));
    let f = Identifier(Id::new(Name { name: intern(op), uid: 0 }, typ, box []));
    Apply(box Apply(box f, box lhs), box rhs)
}

fn true_expr() -> Expr<Id<Name>> { 
    Identifier(Id::new(Name { uid: 0, name: intern("True") }, bool_type(), box []))
}

struct ArgIterator<'a> {
    typ: &'a Type
}
impl <'a> Iterator<&'a Type> for ArgIterator<'a> {
    fn next(&mut self) -> Option<&'a Type> {
        use types::try_get_function;
        match try_get_function(self.typ) {
            Some((arg, rest)) => {
                self.typ = rest;
                Some(arg)
            }
            None => None
        }
    }
}
fn extract_applied_type<'a>(typ: &'a Type) -> &'a Type {
    match typ {
        &TypeApplication(ref lhs, _) => extract_applied_type(*lhs),
        _ => typ
    }
}