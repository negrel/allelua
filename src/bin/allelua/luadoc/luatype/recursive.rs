use core::fmt;
use std::{
    collections::HashMap,
    hash::Hash,
    ops::Deref,
    rc::{Rc, Weak},
};

/// Ref define a reference counting pointer with recursive/cyclic types support.
/// It is a simple wrapper around [Rc] and [Weak].
#[derive(Debug, Clone)]
pub enum Ref<T> {
    Strong(Rc<T>),
    Weak(Weak<T>),
}

impl<T> Hash for Ref<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

impl<T> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<T> Eq for Ref<T> {}

impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Ref::Strong(rc) => rc,
            Ref::Weak(_) => panic!("illegal deref of Ref<T>"),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Ref<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: RecursiveEq> RecursiveEq for Ref<T> {
    fn recursive_eq(&self, other: &Self, hset: &mut HashMap<(usize, usize), bool>) -> bool {
        let pair = (self.as_ptr() as usize, other.as_ptr() as usize);
        if let Some(eq) = hset.get(&pair) {
            return *eq;
        }

        // We consider self and other equal until proven different.
        hset.insert(pair, true);

        let eq = match (self.try_get(), other.try_get()) {
            (Some(lhs), Some(rhs)) => lhs.recursive_eq(&rhs, hset),
            _ => false,
        };

        // Store result to avoid comparing again.
        hset.insert(pair, eq);
        eq
    }
}

impl<T> From<Rc<T>> for Ref<T> {
    fn from(value: Rc<T>) -> Self {
        Self::Strong(value)
    }
}

impl<T> From<Weak<T>> for Ref<T> {
    fn from(value: Weak<T>) -> Self {
        Self::Weak(value)
    }
}

impl<T> Ref<T> {
    pub fn as_ptr(&self) -> *const T {
        match self {
            Ref::Strong(rc) => Rc::as_ptr(rc),
            Ref::Weak(w) => w.as_ptr(),
        }
    }

    pub fn get(&self) -> Rc<T> {
        match self {
            Ref::Strong(rc) => rc.to_owned(),
            Ref::Weak(w) => w.upgrade().expect("failed to upgrade weak reference"),
        }
    }

    pub fn try_get(&self) -> Option<Rc<T>> {
        match self {
            Ref::Strong(rc) => Some(rc.to_owned()),
            Ref::Weak(w) => w.upgrade(),
        }
    }
}

/// RecursiveEq is a custom equality trait that supports check on recursive types.
pub trait RecursiveEq {
    fn recursive_eq(&self, other: &Self, hset: &mut HashMap<(usize, usize), bool>) -> bool;
}
