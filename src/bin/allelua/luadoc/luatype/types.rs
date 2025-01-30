use core::fmt;
use std::rc::Rc;

use super::{RecursiveEq, Ref, TypeCheckError};

/// Type is implemented by all types in our type system.
pub trait Type: RecursiveEq + fmt::Debug + fmt::Display + Into<TypeRef> {
    fn can_assign(&self, other: TypeRef) -> Result<(), TypeCheckError>;
    fn get_field_type(&self, field_name: TypeRef) -> Option<TypeRef>;
    fn normalize(&self) -> TypeRef;
}

/// TypeRef define a reference to one of our [Type] implementation.
#[derive(Debug)]
pub enum TypeRef {
    Never(Ref<NeverType>),
    Literal(Ref<LiteralType>),
    Primitive(Ref<PrimitiveType>),
    Any(Ref<AnyType>),
}

impl RecursiveEq for TypeRef {
    fn recursive_eq(
        &self,
        other: &Self,
        hset: &mut std::collections::HashMap<(usize, usize), bool>,
    ) -> bool {
        match (self, other) {
            (TypeRef::Never(lhs), TypeRef::Never(rhs)) => lhs.recursive_eq(rhs, hset),
            (TypeRef::Literal(lhs), TypeRef::Literal(rhs)) => lhs.recursive_eq(rhs, hset),
            (TypeRef::Primitive(lhs), TypeRef::Primitive(rhs)) => lhs.recursive_eq(rhs, hset),
            (TypeRef::Any(lhs), TypeRef::Any(rhs)) => lhs.recursive_eq(rhs, hset),
            _ => false,
        }
    }
}

macro_rules! forward_type_ref_method_call {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            TypeRef::Never(this) => this.$method($($arg),*),
            TypeRef::Literal(this) => this.$method($($arg),*),
            TypeRef::Primitive(this) => this.$method($($arg),*),
            TypeRef::Any(this) => this.$method($($arg),*),
        }
    };
}

impl Type for TypeRef {
    fn can_assign<T: Type>(&self, other: &T) -> Result<(), TypeCheckError> {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            TypeCheckError::IncompatibleTypes {
                source_type: (),
                target_type: (),
            }
        }

        forward_type_ref_method_call!(self, can_assign, other)
    }

    fn get_field_type<T: Type>(&self, field_name: &T) -> Option<TypeRef> {
        forward_type_ref_method_call!(self, get_field_type, field_name)
    }

    fn normalize(&self) -> TypeRef {
        forward_type_ref_method_call!(self, normalize)
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        forward_type_ref_method_call!(self, fmt, f)
    }
}

/// NeverType define the `never` type in our type system.
#[derive(Debug, PartialEq, Eq)]
pub struct NeverType;

impl RecursiveEq for NeverType {
    fn recursive_eq(
        &self,
        _other: &Self,
        _hset: &mut std::collections::HashMap<(usize, usize), bool>,
    ) -> bool {
        true
    }
}

impl From<Ref<NeverType>> for TypeRef {
    fn from(value: Ref<NeverType>) -> Self {
        TypeRef::Never(value)
    }
}

impl fmt::Display for NeverType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("never")
    }
}

/// AnyType define the `any` type in our type system.
#[derive(Debug, PartialEq, Eq)]
pub struct AnyType;

impl RecursiveEq for AnyType {
    fn recursive_eq(
        &self,
        _other: &Self,
        _hset: &mut std::collections::HashMap<(usize, usize), bool>,
    ) -> bool {
        true
    }
}

impl From<Ref<AnyType>> for TypeRef {
    fn from(value: Ref<AnyType>) -> Self {
        TypeRef::Any(value)
    }
}

impl Type for AnyType {
    fn can_assign<T: Type>(&self, _other: &T) -> Result<(), TypeCheckError> {
        Ok(())
    }

    fn get_field_type<T: Type>(&self, _field_name: &T) -> Option<TypeRef> {
        Some(TypeRef::Any(Rc::new(AnyType).into()))
    }

    fn normalize(&self) -> TypeRef {
        TypeRef::Any(Rc::new(AnyType).into())
    }
}

impl fmt::Display for AnyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("any")
    }
}

/// LiteralType define type of a lua literal.
#[derive(Debug)]
pub struct LiteralType {
    lit: String,
    primitive: Ref<PrimitiveType>,
}

impl RecursiveEq for LiteralType {
    fn recursive_eq(
        &self,
        other: &Self,
        hset: &mut std::collections::HashMap<(usize, usize), bool>,
    ) -> bool {
        self.lit == other.lit && self.primitive.recursive_eq(&other.primitive, hset)
    }
}

impl From<Ref<LiteralType>> for TypeRef {
    fn from(value: Ref<LiteralType>) -> Self {
        TypeRef::Literal(value)
    }
}

impl Type for LiteralType {
    fn can_assign<T: Type>(&self, other: &T) -> Result<(), TypeCheckError> {
        if self.lit == other.lit && self.primitive == other.primitive {
            Ok(())
        } else {
            Err(TypeCheckError::IncompatibleTypes {
                source_type: other,
                target_type: self,
            })
        }
    }

    fn get_field_type<T: Type>(&self, field_name: &T) -> Option<TypeRef> {
        todo!()
    }

    fn normalize(&self) -> TypeRef {
        todo!()
    }
}

impl fmt::Display for LiteralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.lit)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrimitiveType {
    Nil,
    Boolean,
    Number,
    String,
}

impl RecursiveEq for PrimitiveType {
    fn recursive_eq(
        &self,
        other: &Self,
        _hset: &mut std::collections::HashMap<(usize, usize), bool>,
    ) -> bool {
        self == other
    }
}

impl From<Ref<PrimitiveType>> for TypeRef {
    fn from(value: Ref<PrimitiveType>) -> Self {
        TypeRef::Primitive(value)
    }
}

impl Type for PrimitiveType {
    fn can_assign<T: Type>(&self, other: &T) -> Result<(), TypeCheckError> {
        todo!()
    }

    fn get_field_type<T: Type>(&self, field_name: &T) -> Option<TypeRef> {
        todo!()
    }

    fn normalize(&self) -> TypeRef {
        todo!()
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
