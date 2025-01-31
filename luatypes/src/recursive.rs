use std::{
    collections::BTreeMap,
    hash::Hash,
    ops::Deref,
    rc::{Rc, Weak},
};

/// Ref define a reference counting pointer with recursive/cyclic types support.
/// It is a simple wrapper around [Rc] and [Weak]. In order to prevent infinite
/// loop, implementations of [Eq], [Ord] and [Hash] is applied on the underlying pointer
/// and not the value it points to.
#[derive(Debug)]
pub enum Ref<T> {
    Strong(Rc<T>),
    Weak(Weak<T>),
}

// Ref implements Deref for T. Note that deref a Weak reference is considered
// illegal and will panic!
impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Ref::Strong(rc) => rc,
            Ref::Weak(_) => panic!("illegal deref of Ref<T>"),
        }
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Strong(r) => Self::Strong(r.clone()),
            Self::Weak(r) => Self::Weak(r.clone()),
        }
    }
}

impl<T> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<T> Eq for Ref<T> {}

impl<T> PartialOrd for Ref<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Ref<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl<T> Hash for Ref<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
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

impl<T> From<T> for Ref<T> {
    fn from(value: T) -> Self {
        Self::from(Rc::new(value))
    }
}

impl<T> Ref<T> {
    pub fn new_cyclic<F>(cb: F) -> Self
    where
        F: FnOnce(Ref<T>) -> T,
    {
        Self::from(Rc::new_cyclic(|w| cb(Self::from(w.to_owned()))))
    }

    /// Returns underlying pointer.
    pub fn as_ptr(&self) -> *const T {
        match self {
            Ref::Strong(rc) => Rc::as_ptr(rc),
            Ref::Weak(w) => w.as_ptr(),
        }
    }

    /// Retrieves an owned Rc and returns it. This function panics if weak reference
    /// can't be upgraded.
    pub fn get(&self) -> Rc<T> {
        match self {
            Ref::Strong(rc) => rc.to_owned(),
            Ref::Weak(w) => w.upgrade().expect("failed to upgrade weak reference"),
        }
    }

    /// Same as get() but returns an option instead of panicking.
    pub fn try_get(&self) -> Option<Rc<T>> {
        match self {
            Ref::Strong(rc) => Some(rc.to_owned()),
            Ref::Weak(w) => w.upgrade(),
        }
    }
}

pub fn eq<T: RecursiveEq>(lhs: Ref<T>, rhs: Ref<T>) -> bool {
    Context::default().recursive(lhs, rhs)
}

/// Trait for comparisons corresponding to equivalence relation. It is similar
/// to [Eq] trait but supports infinitely recursive structure thanks to [Context].
///
/// Example
/// ```rust
///
/// ```
pub trait RecursiveEq: Sized + std::cmp::Ord {
    fn recursive_eq(this: Ref<Self>, other: Ref<Self>, ctx: &mut Context<Self>) -> bool;
}

/// RecursiveEq context.
#[derive(Debug)]
pub struct Context<T> {
    map: BTreeMap<(Ref<T>, Ref<T>), bool>,
}

impl<T> Default for Context<T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<T: RecursiveEq> Context<T> {
    pub fn recursive(&mut self, mut lhs: Ref<T>, mut rhs: Ref<T>) -> bool {
        if lhs > rhs {
            std::mem::swap(&mut lhs, &mut rhs);
        }

        let k = (lhs.clone(), rhs.clone());

        // Cached result.
        if let Some(eq) = self.map.get(&k) {
            return *eq;
        }

        // Store true to prevent infinite loop.
        self.map.insert(k.clone(), true);
        // Perform actual equality check.
        let eq = T::recursive_eq(lhs, rhs, self);
        // Store result.
        self.map.insert(k, eq);

        eq
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{default::Default, hash::Hasher};

    #[test]
    fn ref_eq() {
        let r1 = Ref::from(true);
        let r2 = Ref::from(true);

        assert!(r1 != r2);
        assert!(r1 == r1);
    }

    #[test]
    fn ref_hash() {
        let r1 = Ref::from(true);
        let r2 = Ref::from(true);

        let mut state1 = std::hash::DefaultHasher::default();
        r1.hash(&mut state1);

        let mut state2 = std::hash::DefaultHasher::default();
        r2.hash(&mut state2);

        assert!(state1.finish() != state2.finish());
    }

    #[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct NonRecursiveT {
        b: bool,
    }

    impl RecursiveEq for NonRecursiveT {
        fn recursive_eq(
            this: Ref<Self>,
            other: Ref<Self>,
            _ctx: &mut super::Context<Self>,
        ) -> bool {
            this.b == other.b
        }
    }

    #[test]
    fn recursive_eq_returns_true_for_equal_non_recursive_types() {
        let v1 = NonRecursiveT { b: true };
        let v2 = NonRecursiveT { b: true };

        assert!(super::eq(v1.into(), v2.into()));
    }

    #[test]
    fn recursive_eq_returns_false_for_not_equal_non_recursive_types() {
        let v1 = NonRecursiveT { b: true };
        let v2 = NonRecursiveT { b: false };

        assert!(!super::eq(v1.into(), v2.into()));
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct RecursiveT {
        i: usize,
        rec_ref: Option<Ref<Self>>,
    }

    impl RecursiveEq for RecursiveT {
        fn recursive_eq(this: Ref<Self>, other: Ref<Self>, ctx: &mut super::Context<Self>) -> bool {
            if this.i != other.i {
                false
            } else {
                match (this.rec_ref.clone(), other.rec_ref.clone()) {
                    (Some(lhs), Some(rhs)) => ctx.recursive(lhs, rhs),
                    (None, None) => true,
                    _ => false,
                }
            }
        }
    }

    #[test]
    fn recursive_eq_returns_true_for_equal_recursive_types() {
        let v1 = Ref::new_cyclic(|w| RecursiveT {
            i: 1,
            rec_ref: Some(w),
        });
        let v2 = Ref::new_cyclic(|w| RecursiveT {
            i: 1,
            rec_ref: Some(w),
        });

        assert!(super::eq(v1.clone(), v2));
        assert!(super::eq(v1.clone(), v1));
    }

    #[test]
    fn recursive_eq_returns_false_for_equal_recursive_types() {
        let v1 = Ref::new_cyclic(|w| RecursiveT {
            i: 1,
            rec_ref: Some(w),
        });
        let v2 = Ref::new_cyclic(|w| RecursiveT {
            i: 2,
            rec_ref: Some(w),
        });

        assert!(!super::eq(v1, v2))
    }
}
