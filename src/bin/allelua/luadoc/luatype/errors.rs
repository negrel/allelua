use super::TypeRef;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum TypeCheckError {
    #[error("type {:?} is not assignable to {:?}", source_type.to_string(), target_type.to_string())]
    IncompatibleTypes {
        source_type: TypeRef,
        target_type: TypeRef,
    },
}
