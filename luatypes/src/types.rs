use std::{
    collections::BTreeMap,
    fmt::{self},
    ops::Deref,
};

use smol_str::SmolStr;

use crate::cyclic::{self, Ref};

/// Checks that type `assignee` is assignable to type `t`.
pub fn can_assign(t: Type, assignee: Type) -> bool {
    cyclic::op(Type::can_assign, (t, assignee), true)
}

/// Type represents a type in our type-system.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Never(NeverType),
    Any(AnyType),
    Primitive(PrimitiveType),
    Literal(LiteralType),
    Union(UnionType),
    Intersection(IntersectionType),
    Interface(InterfaceType),
    Named(NamedType),
    Ref(Ref<Type>),
}

impl From<Ref<Type>> for Type {
    fn from(value: Ref<Type>) -> Self {
        Self::Ref(value)
    }
}

impl Type {
    fn can_assign(ctx: &mut cyclic::Context<(Type, Type), bool>, (lhs, rhs): (Type, Type)) -> bool {
        match (lhs.clone(), rhs.clone()) {
            // Nothing can be assigned to the never type except the never type
            // itself.
            (Type::Never(_), Type::Never(_)) => true,
            // Anything can be assigned to any.
            (Type::Any(_), _) => true,
            // Primitive can be assigned if they're equal.
            (Type::Primitive(lhs), Type::Primitive(rhs)) => lhs == rhs,
            // Literal can be assigned if they're equal.
            (Type::Literal(lhs), Type::Literal(rhs)) => lhs.can_assign(&rhs),
            // Literal can be assigned to primitive of same type.
            (Type::Primitive(lhs), Type::Literal(rhs)) => lhs == rhs.primitive,
            // Union.
            (Type::Union(l), _) => l.can_assign(lhs, rhs, ctx),
            // Intersection.
            (Type::Intersection(l), _) => l.can_assign(lhs, rhs, ctx),
            // Interface.
            (Type::Interface(l), _) => l.can_assign(lhs, rhs, ctx),
            // Named type.
            (Type::Named(l), _) => l.can_assign(lhs, rhs, ctx),
            (l, Type::Named(r)) => {
                ctx.cyclic(Type::can_assign, (l, r.alias.upgrade().deref().clone()))
            }
            // Ref types.
            (Type::Ref(lhs), rhs) => {
                ctx.cyclic(Type::can_assign, (lhs.upgrade().deref().clone(), rhs))
            }
            (lhs, Type::Ref(rhs)) => {
                ctx.cyclic(Type::can_assign, (lhs, rhs.upgrade().deref().clone()))
            }
            // Anything else is false.
            _ => false,
        }
    }

    pub fn field(&self, k: &Type) -> Type {
        match self {
            Type::Union(u) => u.field(k),
            Type::Intersection(i) => i.field(k),
            Type::Interface(i) => i.field(k),
            Type::Named(n) => n.field(k),
            Type::Ref(r) => r.upgrade().deref().field(k),
            _ => Type::Primitive(PrimitiveType::Nil),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Never(_) => f.write_str("never"),
            Type::Any(_) => f.write_str("any"),
            Type::Primitive(prim) => fmt::Display::fmt(prim, f),
            Type::Literal(lit) => fmt::Display::fmt(lit, f),
            Type::Union(u) => fmt::Display::fmt(u, f),
            Type::Intersection(i) => fmt::Display::fmt(i, f),
            Type::Interface(i) => fmt::Display::fmt(i, f),
            Type::Named(n) => fmt::Display::fmt(n, f),
            Type::Ref(r) => match r {
                Ref::Strong(r) => fmt::Display::fmt(r.deref(), f),
                Ref::Weak(w) => write!(f, "0x{:x}", w.as_ptr() as usize),
            },
        }
    }
}

/// NeverType define the `never` type in our type system.
/// Nothing can be assigned to the never type has it requires all the possible
/// properties. On the other hand, variable of type never can be assigned to
/// anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NeverType;

impl From<NeverType> for Type {
    fn from(value: NeverType) -> Self {
        Type::Never(value)
    }
}

/// AnyType define the `any` type in our type system.
/// Everything can be assigned to the never type has it doesn't requires any
/// properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnyType;

impl From<AnyType> for Type {
    fn from(value: AnyType) -> Self {
        Type::Any(value)
    }
}

/// PrimitiveType define a Lua primitive type. Lua primitives are `nil`, `boolean`,
/// `number` and `string`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrimitiveType {
    Nil,
    Boolean,
    Number,
    String,
}

impl From<PrimitiveType> for Type {
    fn from(value: PrimitiveType) -> Self {
        Type::Primitive(value)
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            PrimitiveType::Nil => "nil",
            PrimitiveType::Boolean => "boolean",
            PrimitiveType::Number => "number",
            PrimitiveType::String => "string",
        };
        f.write_str(str)
    }
}

/// LiteralType define type of a Lua literal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LiteralType {
    lit: SmolStr,
    primitive: PrimitiveType,
}

impl From<LiteralType> for Type {
    fn from(value: LiteralType) -> Self {
        Type::Literal(value)
    }
}

impl fmt::Display for LiteralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.primitive {
            PrimitiveType::Nil | PrimitiveType::Boolean | PrimitiveType::Number => {
                f.write_str(&self.lit)
            }
            PrimitiveType::String => write!(f, "\"{}\"", self.lit),
        }
    }
}

impl LiteralType {
    /// Creates a new literal string type from the given string.
    pub fn string(lit: impl Into<SmolStr>) -> Self {
        Self {
            lit: lit.into(),
            primitive: PrimitiveType::String,
        }
    }

    /// Creates a new literal number type from the given string.
    pub fn number(lit: impl Into<SmolStr>) -> Self {
        Self {
            lit: lit.into(),
            primitive: PrimitiveType::Number,
        }
    }

    fn can_assign(&self, rhs: &LiteralType) -> bool {
        // TODO: fix lit comparison for float numbers as they're approximation
        // of numbers.
        self.lit == rhs.lit && self.primitive == rhs.primitive
    }
}

/// UnionType define a union of types. All types that can be assigned to one of
/// union's variant type can be assigned to the union.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnionType {
    variants: Vec<Type>,
}

impl From<UnionType> for Type {
    fn from(value: UnionType) -> Self {
        Type::Union(value)
    }
}

impl From<Type> for UnionType {
    fn from(value: Type) -> Self {
        Self::from(vec![value])
    }
}

impl From<Vec<Type>> for UnionType {
    fn from(value: Vec<Type>) -> Self {
        Self { variants: value }
    }
}

impl fmt::Display for UnionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("(")?;
        }

        f.write_str(
            &self
                .variants
                .clone()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" | "),
        )?;

        if f.alternate() {
            f.write_str("(")?;
        }

        Ok(())
    }
}

impl UnionType {
    fn can_assign(
        &self,
        this: Type,
        rhs: Type,
        ctx: &mut cyclic::Context<(Type, Type), bool>,
    ) -> bool {
        match rhs {
            Type::Primitive(_) | Type::Literal(_) => {
                for v in self.variants.iter() {
                    if Type::can_assign(ctx, (v.clone(), rhs.clone())) {
                        return true;
                    }
                }
                false
            }
            Type::Union(UnionType { variants }) => {
                for v in variants.iter() {
                    if !Type::can_assign(ctx, (this.clone(), v.clone())) {
                        return false;
                    }
                }
                true
            }
            Type::Intersection(IntersectionType { variants }) => {
                for v in variants.iter() {
                    if Type::can_assign(ctx, (this.clone(), v.clone())) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn field(&self, k: &Type) -> Type {
        let mut result = Vec::new();
        for v in self.variants.iter() {
            let f = v.field(k);
            match f {
                Type::Primitive(PrimitiveType::Nil) => {}
                _ => result.push(f),
            }
        }

        if result.len() <= 1 {
            result.pop().unwrap_or(Type::Primitive(PrimitiveType::Nil))
        } else {
            Type::from(Self::from(result))
        }
    }
}

/// IntersectionType define an intersection of types. All types that can be
/// assigned to all of intersection's variant type can be assigned to the
/// intersection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntersectionType {
    variants: Vec<Type>,
}

impl From<IntersectionType> for Type {
    fn from(value: IntersectionType) -> Self {
        Type::Intersection(value)
    }
}

impl From<Type> for IntersectionType {
    fn from(value: Type) -> Self {
        Self::from(vec![value])
    }
}

impl From<Vec<Type>> for IntersectionType {
    fn from(value: Vec<Type>) -> Self {
        Self { variants: value }
    }
}

impl fmt::Display for IntersectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("(")?;
        }

        f.write_str(
            &self
                .variants
                .clone()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" & "),
        )?;

        if f.alternate() {
            f.write_str(")")?;
        }

        Ok(())
    }
}

impl IntersectionType {
    fn can_assign(
        &self,
        this: Type,
        rhs: Type,
        ctx: &mut cyclic::Context<(Type, Type), bool>,
    ) -> bool {
        match rhs {
            Type::Primitive(_) | Type::Literal(_) => {
                for v in self.variants.iter() {
                    if !Type::can_assign(ctx, (v.clone(), rhs.clone())) {
                        return false;
                    }
                }
                true
            }
            Type::Union(UnionType { variants }) => {
                for v in variants.iter() {
                    if !Type::can_assign(ctx, (this.clone(), v.clone())) {
                        return false;
                    }
                }
                true
            }
            Type::Intersection(IntersectionType { variants }) => {
                for v in variants.iter() {
                    if !Type::can_assign(ctx, (this.clone(), v.clone())) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn field(&self, k: &Type) -> Type {
        let mut result = Vec::new();
        for v in self.variants.iter() {
            let f = v.field(k);
            match f {
                Type::Primitive(PrimitiveType::Nil) => {}
                _ => result.push(f),
            }
        }

        if result.len() <= 1 {
            result.pop().unwrap_or(Type::Primitive(PrimitiveType::Nil))
        } else {
            Type::from(Self::from(result))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InterfaceType {
    fields: BTreeMap<Type, Type>,
}
impl InterfaceType {
    fn can_assign(
        &self,
        _this: Type,
        rhs: Type,
        ctx: &mut cyclic::Context<(Type, Type), bool>,
    ) -> bool {
        for (k, v) in self.fields.iter() {
            let f = rhs.field(k);
            if !Type::can_assign(ctx, (v.clone(), f)) {
                return false;
            }
        }

        true
    }

    fn field(&self, k: &Type) -> Type {
        self.fields
            .get(k)
            .cloned()
            .unwrap_or(Type::Primitive(PrimitiveType::Nil))
    }
}

impl From<InterfaceType> for Type {
    fn from(value: InterfaceType) -> Self {
        Type::Interface(value)
    }
}

impl<const N: usize> From<[(Type, Type); N]> for InterfaceType {
    fn from(value: [(Type, Type); N]) -> Self {
        Self {
            fields: BTreeMap::from(value),
        }
    }
}

impl fmt::Display for InterfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("{\n")?;
        } else {
            f.write_str("{ ")?;
        }

        // TODO: properly handle literal string.
        let iter = self.fields.iter();
        let len = iter.len();
        for (i, (k, v)) in iter.enumerate() {
            f.write_str(&k.to_string())?;
            f.write_str(" = ")?;
            f.write_str(&v.to_string())?;

            if i + 1 < len {
                if f.alternate() {
                    f.write_str("\n")?;
                } else {
                    f.write_str(", ")?;
                }
            }
        }

        if f.alternate() {
            f.write_str("}")?;
        } else {
            f.write_str(" }")?;
        }

        Ok(())
    }
}

/// NamedType define a custom type with a name. This is the prerequisite to
/// cyclic types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NamedType {
    name: String,
    alias: Ref<Type>,
}

impl fmt::Display for NamedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl NamedType {
    /// Creates a new non-cylic named type.
    pub fn new(name: String, r#type: Type) -> Self {
        Self {
            name,
            alias: r#type.into(),
        }
    }

    /// Creates a new cyclic [NamedType] while giving you a weak [Ref] to the
    /// allocation, to allow you to construct a Type which holds a weak pointer to
    /// itself.
    ///
    /// Generally, a structure circularly referencing itself, either directly or
    /// indirectly, should not hold a strong reference to itself to prevent a
    /// memory leak. Using this function, you get access to the weak pointer
    /// during the initialization of T, before the underlying Rc<T> is created,
    /// such that you can clone and store it inside the T.
    ///
    /// Since the new underlying Rc<T> is not fully-constructed until
    /// [new_cyclic] returns, calling [upgrade] on the weak reference inside your
    /// closure will panic.
    pub fn new_cyclic<F>(name: String, cb: F) -> Self
    where
        F: FnOnce(Ref<Type>) -> Type,
    {
        Self {
            name,
            alias: Ref::new_cyclic(cb),
        }
    }

    pub fn can_assign(
        &self,
        _this: Type,
        rhs: Type,
        ctx: &mut cyclic::Context<(Type, Type), bool>,
    ) -> bool {
        ctx.cyclic(Type::can_assign, (self.alias.deref().clone(), rhs))
    }

    fn field(&self, k: &Type) -> Type {
        Type::field(self.alias.deref(), k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        assert!(can_assign(never.clone(), never.clone()));
        assert!(!can_assign(never.clone(), any));
        assert!(!can_assign(never.clone(), nil));
        assert!(!can_assign(never.clone(), boolean));
        assert!(!can_assign(never.clone(), number));
        assert!(!can_assign(never.clone(), string));
        assert!(!can_assign(never.clone(), union_num_str));
        assert!(!can_assign(never.clone(), union_num_nil));
        assert!(!can_assign(
            never.clone(),
            inter_union_num_str_union_num_nil
        ));
        assert!(!can_assign(never.clone(), iface_foo_str));
        assert!(!can_assign(never.clone(), iface_foo_str_bar_num));
    }

    #[test]
    fn any_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        assert!(can_assign(any.clone(), never.clone()));
        assert!(can_assign(any.clone(), any.clone()));
        assert!(can_assign(any.clone(), nil.clone()));
        assert!(can_assign(any.clone(), boolean.clone()));
        assert!(can_assign(any.clone(), number.clone()));
        assert!(can_assign(any.clone(), string.clone()));
        assert!(can_assign(any.clone(), union_num_str.clone()));
        assert!(can_assign(any.clone(), union_num_nil.clone()));
        assert!(can_assign(
            any.clone(),
            inter_union_num_str_union_num_nil.clone()
        ));
        assert!(can_assign(any.clone(), iface_foo_str));
        assert!(can_assign(any.clone(), iface_foo_str_bar_num));
    }

    #[test]
    fn primitive_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        for (i, lhs) in [nil.clone(), boolean.clone(), number.clone(), string.clone()]
            .iter()
            .enumerate()
        {
            for (j, rhs) in [nil.clone(), boolean.clone(), number.clone(), string.clone()]
                .iter()
                .enumerate()
            {
                if i == j {
                    assert!(can_assign(lhs.clone(), rhs.clone()));
                    assert!(can_assign(rhs.clone(), lhs.clone()));
                } else {
                    assert!(!can_assign(lhs.clone(), rhs.clone()));
                    assert!(!can_assign(rhs.clone(), lhs.clone()));
                }
            }

            assert!(!can_assign(lhs.clone(), any.clone()));
            assert!(!can_assign(lhs.clone(), never.clone()));
            assert!(!can_assign(lhs.clone(), union_num_str.clone()));
            assert!(!can_assign(lhs.clone(), union_num_nil.clone()));
            assert!(!can_assign(
                lhs.clone(),
                inter_union_num_str_union_num_nil.clone()
            ));
            assert!(!can_assign(lhs.clone(), iface_foo_str.clone()));
            assert!(!can_assign(lhs.clone(), iface_foo_str_bar_num.clone()));
        }
    }

    #[test]
    fn union_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        assert!(!can_assign(union_num_str.clone(), nil.clone()));
        assert!(!can_assign(union_num_str.clone(), boolean.clone()));
        assert!(!can_assign(union_num_str.clone(), any.clone()));
        assert!(!can_assign(union_num_str.clone(), never.clone()));
        assert!(can_assign(union_num_str.clone(), number.clone()));
        assert!(can_assign(union_num_str.clone(), string.clone()));
        assert!(can_assign(union_num_str.clone(), union_num_str.clone()));
        assert!(!can_assign(union_num_str.clone(), union_num_nil.clone()));
        assert!(can_assign(
            union_num_str.clone(),
            inter_union_num_str_union_num_nil.clone()
        ));
        assert!(!can_assign(union_num_str.clone(), iface_foo_str));
        assert!(!can_assign(union_num_str.clone(), iface_foo_str_bar_num));
    }

    #[test]
    fn intersection_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            nil.clone()
        ));
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            boolean.clone()
        ));
        assert!(!can_assign(inter_union_num_str_union_num_nil.clone(), any));
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            never.clone()
        ));
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            string.clone()
        ));
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            union_num_str.clone()
        ));
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            union_num_nil.clone()
        ));

        // Only number can be assigned as it is present in both union.
        assert!(can_assign(
            inter_union_num_str_union_num_nil.clone(),
            number.clone()
        ));

        // This doesn't work unless we normalize the intersection.
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            inter_union_num_str_union_num_nil.clone()
        ));

        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            iface_foo_str
        ));
        assert!(!can_assign(
            inter_union_num_str_union_num_nil.clone(),
            iface_foo_str_bar_num
        ));
    }

    #[test]
    fn interface_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        assert!(!can_assign(iface_foo_str.clone(), never));
        assert!(!can_assign(iface_foo_str.clone(), any));
        assert!(!can_assign(iface_foo_str.clone(), nil));
        assert!(!can_assign(iface_foo_str.clone(), boolean));
        assert!(!can_assign(iface_foo_str.clone(), number));
        assert!(!can_assign(iface_foo_str.clone(), string));
        assert!(!can_assign(iface_foo_str.clone(), union_num_str));
        assert!(!can_assign(iface_foo_str.clone(), union_num_nil));
        assert!(!can_assign(
            iface_foo_str.clone(),
            inter_union_num_str_union_num_nil
        ));

        assert!(can_assign(iface_foo_str.clone(), iface_foo_str.clone()));
        assert!(can_assign(
            iface_foo_str.clone(),
            iface_foo_str_bar_num.clone()
        ));
        assert!(can_assign(
            iface_foo_str_bar_num.clone(),
            iface_foo_str_bar_num.clone()
        ));
        assert!(!can_assign(
            iface_foo_str_bar_num.clone(),
            iface_foo_str.clone()
        ));
    }

    #[test]
    fn named_can_assign() {
        let never = Type::Never(NeverType);
        let any = Type::Any(AnyType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));
        let iface_foo_str = Type::Interface(InterfaceType::from([(
            Type::from(LiteralType::string("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::string("foo")), string.clone()),
            (Type::from(LiteralType::string("bar")), number.clone()),
        ]));

        let cyclic_iface = Type::Named(NamedType::new_cyclic("List".to_owned(), |w| {
            Type::Interface(InterfaceType::from([
                (Type::from(LiteralType::string("nil")), nil.clone()),
                (Type::from(LiteralType::string("value")), number.clone()),
                (Type::from(LiteralType::string("next")), w.into()),
            ]))
        }));

        let cyclic_iface2 = Type::Named(NamedType::new_cyclic("List".to_owned(), |w| {
            Type::Interface(InterfaceType::from([
                (Type::from(LiteralType::string("value")), number.clone()),
                (Type::from(LiteralType::string("next")), w.into()),
                (Type::from(LiteralType::string("id")), string.clone()),
            ]))
        }));

        assert!(!can_assign(cyclic_iface.clone(), never.clone()));
        assert!(!can_assign(cyclic_iface.clone(), any.clone()));
        assert!(!can_assign(cyclic_iface.clone(), nil.clone()));
        assert!(!can_assign(cyclic_iface.clone(), boolean.clone()));
        assert!(!can_assign(cyclic_iface.clone(), number.clone()));
        assert!(!can_assign(cyclic_iface.clone(), string.clone()));
        assert!(!can_assign(cyclic_iface.clone(), union_num_str.clone()));
        assert!(!can_assign(cyclic_iface.clone(), union_num_nil.clone()));
        assert!(!can_assign(
            cyclic_iface.clone(),
            inter_union_num_str_union_num_nil.clone()
        ));
        assert!(!can_assign(cyclic_iface.clone(), iface_foo_str.clone()));
        assert!(!can_assign(
            cyclic_iface.clone(),
            iface_foo_str_bar_num.clone()
        ));
        assert!(can_assign(cyclic_iface.clone(), cyclic_iface.clone()));
        assert!(can_assign(cyclic_iface.clone(), cyclic_iface2.clone()));
        assert!(can_assign(cyclic_iface2.clone(), cyclic_iface2.clone()));
        assert!(!can_assign(cyclic_iface2.clone(), cyclic_iface.clone()));
    }
}
