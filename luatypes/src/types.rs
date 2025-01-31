use std::fmt;

/// Type represents a type in our type-system.
#[derive(Debug, Clone)]
pub enum Type {
    Never(NeverType),
    Any(AnyType),
    Primitive(PrimitiveType),
    Literal(LiteralType),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Never(_) => f.write_str("never"),
            Type::Any(_) => f.write_str("any"),
            Type::Primitive(prim) => fmt::Display::fmt(prim, f),
            Type::Literal(lit) => fmt::Display::fmt(lit, f),
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

/// AnyType define the `any` type in our type system.
/// Everything can be assigned to the never type has it doesn't requires any
/// properties.
#[derive(Debug, Clone, Copy)]
pub struct AnyType;

/// PrimitiveType define a Lua primitive type. Lua primitives are `nil`, `boolean`,
/// `number` and `string`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
    Nil,
    Boolean,
    Number,
    String,
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

impl fmt::Display for LiteralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.lit)
    }
}

impl LiteralType {
    pub fn can_assign(&self, rhs: &LiteralType) -> bool {
        // TODO: fix lit comparison for float numbers as they're approximation
        // of numbers.
        self.lit == rhs.lit && self.primitive == rhs.primitive
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

        assert!(never.can_assign(&never));
        assert!(!never.can_assign(&any));
        assert!(!never.can_assign(&nil));
        assert!(!never.can_assign(&boolean));
        assert!(!never.can_assign(&number));
        assert!(!never.can_assign(&string));
    }

    #[test]
    fn any_can_assign() {
        let any = Type::Any(AnyType);
        let never = Type::Never(NeverType);
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);

        assert!(any.can_assign(&never));
        assert!(any.can_assign(&any));
        assert!(any.can_assign(&nil));
        assert!(any.can_assign(&boolean));
        assert!(any.can_assign(&number));
        assert!(any.can_assign(&string));
    }

    #[test]
    fn primitive_can_assign() {
        let nil = Type::Primitive(PrimitiveType::Nil);
        let boolean = Type::Primitive(PrimitiveType::Boolean);
        let number = Type::Primitive(PrimitiveType::Number);
        let string = Type::Primitive(PrimitiveType::String);

        let any = Type::Any(AnyType);
        let never = Type::Never(NeverType);

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

            // any and never can't be assigned to a primitive type.
            assert!(!lhs.can_assign(&any));
            assert!(!lhs.can_assign(&never));
        }
    }
}
