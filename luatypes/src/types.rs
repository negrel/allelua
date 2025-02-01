use std::{collections::BTreeMap, fmt};

use crate::cyclic::{self};

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
}

impl Type {
    fn can_assign(ctx: &mut cyclic::Context<(Type, Type), bool>, (lhs, rhs): (Type, Type)) -> bool {
        match (lhs.clone(), rhs.clone()) {
            // Nothing can be assigned to the never type except the never type
            // itself.
            (Type::Never(_), Type::Never(_)) => true,
            (Type::Never(_), _) => false,
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
            // Inteface.
            (Type::Interface(l), _) => l.can_assign(lhs, rhs, ctx),
            // Anything else is false.
            _ => false,
        }
    }

    pub fn field(&self, k: &Type) -> Type {
        match self {
            Type::Union(u) => u.field(k),
            Type::Intersection(i) => i.field(k),
            Type::Interface(i) => i.field(k),
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
    lit: String,
    primitive: PrimitiveType,
}

impl From<LiteralType> for Type {
    fn from(value: LiteralType) -> Self {
        Type::Literal(value)
    }
}

impl fmt::Display for LiteralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.lit)
    }
}

impl LiteralType {
    /// Creates a new literal string type from the given string.
    pub fn string(lit: String) -> Self {
        debug_assert!(
            {
                if lit.len() < 2 {
                    false
                } else {
                    let bytes = lit.as_bytes();
                    [b'`', b'"', b'\''].contains(&bytes[0])
                }
            },
            "invalid literal string, surrounding quotes are missing"
        );

        Self {
            lit,
            primitive: PrimitiveType::String,
        }
    }

    /// Creates a new literal string type by escaping the given string.
    pub fn escape_str(lit: &'static str) -> Self {
        Self::string(format!("{lit:?}"))
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
            result
                .pop()
                .unwrap_or(Type::Primitive(PrimitiveType::Nil).into())
        } else {
            Type::from(Type::from(Self::from(result)))
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
        for (k, v) in self.fields.iter() {
            f.write_str(&k.to_string())?;
            f.write_str(" = ")?;
            f.write_str(&v.to_string())?;
        }

        if f.alternate() {
            f.write_str("}\n")?;
        } else {
            f.write_str(" }")?;
        }

        Ok(())
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
            Type::from(LiteralType::escape_str("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::escape_str("foo")), string.clone()),
            (Type::from(LiteralType::escape_str("bar")), number.clone()),
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
            Type::from(LiteralType::escape_str("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::escape_str("foo")), string.clone()),
            (Type::from(LiteralType::escape_str("bar")), number.clone()),
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
            Type::from(LiteralType::escape_str("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::escape_str("foo")), string.clone()),
            (Type::from(LiteralType::escape_str("bar")), number.clone()),
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
            Type::from(LiteralType::escape_str("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::escape_str("foo")), string.clone()),
            (Type::from(LiteralType::escape_str("bar")), number.clone()),
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
            Type::from(LiteralType::escape_str("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::escape_str("foo")), string.clone()),
            (Type::from(LiteralType::escape_str("bar")), number.clone()),
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
            Type::from(LiteralType::escape_str("foo")),
            string.clone(),
        )]));
        let iface_foo_str_bar_num = Type::Interface(InterfaceType::from([
            (Type::from(LiteralType::escape_str("foo")), string.clone()),
            (Type::from(LiteralType::escape_str("bar")), number.clone()),
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
}
