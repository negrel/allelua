use std::fmt;

/// Type represents a type in our type-system.
#[derive(Debug, Clone)]
pub enum Type {
    Never(NeverType),
    Any(AnyType),
    Primitive(PrimitiveType),
    Literal(LiteralType),
    Union(UnionType),
    Intersection(IntersectionType),
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
        }
    }
}

impl Type {
    /// Checks that `rhs` can be assigned to `self`.
    pub fn can_assign(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            // Nothing can be assigned to the never type except the never type
            // itself.
            (Type::Never(_), Type::Never(_)) => true,
            (Type::Never(_), _) => false,
            // Anything can be assigned to any.
            (Type::Any(_), _) => true,
            // Primitive can be assigned if they're equal.
            (Type::Primitive(lhs), Type::Primitive(rhs)) => lhs == rhs,
            // Literal can be assigned if they're equal.
            (Type::Literal(lhs), Type::Literal(rhs)) => lhs.can_assign(rhs),
            // Literal can be assigned to primitive of same type.
            (Type::Primitive(lhs), Type::Literal(rhs)) => *lhs == rhs.primitive,
            // Union.
            (Type::Union(lhs), rhs) => lhs.can_assign(rhs),
            // Intersection.
            (Type::Intersection(lhs), rhs) => lhs.can_assign(rhs),
            // Anything else is false.
            _ => false,
        }
    }
}

/// NeverType define the `never` type in our type system.
/// Nothing can be assigned to the never type has it requires all the possible
/// properties. On the other hand, variable of type never can be assigned to
/// anything.
#[derive(Debug, Clone, Copy)]
pub struct NeverType;

impl From<NeverType> for Type {
    fn from(value: NeverType) -> Self {
        Type::Never(value)
    }
}

/// AnyType define the `any` type in our type system.
/// Everything can be assigned to the never type has it doesn't requires any
/// properties.
#[derive(Debug, Clone, Copy)]
pub struct AnyType;

impl From<AnyType> for Type {
    fn from(value: AnyType) -> Self {
        Type::Any(value)
    }
}

/// PrimitiveType define a Lua primitive type. Lua primitives are `nil`, `boolean`,
/// `number` and `string`.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
    fn can_assign(&self, rhs: &LiteralType) -> bool {
        // TODO: fix lit comparison for float numbers as they're approximation
        // of numbers.
        self.lit == rhs.lit && self.primitive == rhs.primitive
    }
}

/// UnionType define a union of types. All types that can be assigned to one of
/// union's variant type can be assigned to the union.
#[derive(Debug, Clone)]
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
    fn can_assign(&self, rhs: &Type) -> bool {
        match rhs {
            Type::Primitive(_) | Type::Literal(_) => {
                for v in self.variants.iter() {
                    if v.can_assign(rhs) {
                        return true;
                    }
                }
                false
            }
            Type::Union(UnionType { variants })
            | Type::Intersection(IntersectionType { variants }) => {
                for v in variants.iter() {
                    if !self.can_assign(v) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

/// IntersectionType define an intersection of types. All types that can be
/// assigned to all of intersection's variant type can be assigned to the
/// intersection.
#[derive(Debug, Clone)]
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
    fn can_assign(&self, rhs: &Type) -> bool {
        match rhs {
            Type::Primitive(_) | Type::Literal(_) => {
                for v in self.variants.iter() {
                    if !v.can_assign(rhs) {
                        return false;
                    }
                }
                true
            }
            Type::Union(UnionType { variants })
            | Type::Intersection(IntersectionType { variants }) => {
                for v in variants.iter() {
                    if !self.can_assign(v) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
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

        assert!(never.can_assign(&never));
        assert!(!never.can_assign(&any));
        assert!(!never.can_assign(&nil));
        assert!(!never.can_assign(&boolean));
        assert!(!never.can_assign(&number));
        assert!(!never.can_assign(&string));
        assert!(!never.can_assign(&union_num_str));
        assert!(!never.can_assign(&union_num_nil));
        assert!(!never.can_assign(&inter_union_num_str_union_num_nil));
    }

    #[test]
    fn any_can_assign() {
        let any = Type::Any(AnyType);
        let never = Type::Never(NeverType);
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

        assert!(any.can_assign(&never));
        assert!(any.can_assign(&any));
        assert!(any.can_assign(&nil));
        assert!(any.can_assign(&boolean));
        assert!(any.can_assign(&number));
        assert!(any.can_assign(&string));
        assert!(any.can_assign(&union_num_str));
        assert!(any.can_assign(&union_num_nil));
        assert!(any.can_assign(&inter_union_num_str_union_num_nil));
    }

    #[test]
    fn primitive_can_assign() {
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);

        let any = Type::Any(AnyType);
        let never = Type::Never(NeverType);
        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
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
                    assert!(lhs.can_assign(rhs));
                    assert!(rhs.can_assign(lhs));
                } else {
                    assert!(!lhs.can_assign(rhs));
                    assert!(!rhs.can_assign(lhs));
                }
            }

            assert!(!lhs.can_assign(&any));
            assert!(!lhs.can_assign(&never));
            assert!(!lhs.can_assign(&union_num_str));
            assert!(!lhs.can_assign(&union_num_nil));
            assert!(!lhs.can_assign(&inter_union_num_str_union_num_nil));
        }
    }

    #[test]
    fn union_can_assign() {
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);

        let any = Type::Any(AnyType);
        let never = Type::Never(NeverType);

        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));

        assert!(!union_num_str.can_assign(&nil));
        assert!(!union_num_str.can_assign(&boolean));
        assert!(!union_num_str.can_assign(&any));
        assert!(!union_num_str.can_assign(&never));

        assert!(union_num_str.can_assign(&number));
        assert!(union_num_str.can_assign(&string));
        assert!(union_num_str.can_assign(&union_num_str));

        assert!(!union_num_str.can_assign(&union_num_nil));
        assert!(!union_num_str.can_assign(&inter_union_num_str_union_num_nil));
    }

    #[test]
    fn intersection_can_assign() {
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);

        let any = Type::Any(AnyType);
        let never = Type::Never(NeverType);

        let union_num_str = Type::Union(UnionType::from(vec![number.clone(), string.clone()]));
        let union_num_nil = Type::Union(UnionType::from(vec![number.clone(), nil.clone()]));
        let inter_union_num_str_union_num_nil = Type::Intersection(IntersectionType::from(vec![
            union_num_str.clone(),
            union_num_nil.clone(),
        ]));

        assert!(!inter_union_num_str_union_num_nil.can_assign(&nil));
        assert!(!inter_union_num_str_union_num_nil.can_assign(&boolean));
        assert!(!inter_union_num_str_union_num_nil.can_assign(&any));
        assert!(!inter_union_num_str_union_num_nil.can_assign(&never));
        assert!(!inter_union_num_str_union_num_nil.can_assign(&string));
        assert!(!inter_union_num_str_union_num_nil.can_assign(&union_num_str));
        assert!(!inter_union_num_str_union_num_nil.can_assign(&union_num_nil));

        // Only number can be assigned as it is present in both union.
        assert!(inter_union_num_str_union_num_nil.can_assign(&number));

        // This doesn't work unless we normalize the intersection.
        assert!(!inter_union_num_str_union_num_nil.can_assign(&inter_union_num_str_union_num_nil));
    }
}
