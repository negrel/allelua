use std::{
    collections::BTreeMap,
    hash::Hash,
    ops::Deref,
    rc::{Rc, Weak},
};

/// Ref define a reference counting pointer with cyclic types support.
/// It is a simple wrapper around [Rc] and [Weak]. In order to prevent infinite
/// loop, implementations of [Eq], [Ord] and [Hash] is applied on the underlying pointer
/// and not the value it points to.
#[derive(Debug)]
pub enum Ref<T> {
    Strong(Rc<T>),
    Weak(Weak<T>),
}

// Ref implements Deref for T. Note that deref a Weak reference is illegal and
// will panic!
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

impl<T> TryFrom<Ref<T>> for Rc<T> {
    type Error = Ref<T>;

    fn try_from(value: Ref<T>) -> Result<Self, Self::Error> {
        match value.clone() {
            Ref::Strong(rc) => Ok(rc.to_owned()),
            Ref::Weak(w) => w.upgrade().map(Ok).unwrap_or(Err(value)),
        }
    }
}

impl<T> Ref<T> {
    /// Creates a new cyclic reference while giving you a weak [Ref] to the
    /// allocation, to allow you to construct a T which holds a weak pointer to
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

    /// Upgrades to a strong reference.
    pub fn upgrade(&self) -> Self {
        match self {
            Ref::Strong(_) => self.clone(),
            Ref::Weak(w) => w
                .upgrade()
                .expect("failed to upgrade weak reference")
                .into(),
        }
    }
}

/// Calls cyclic op on the given args using a new [Context]. Pending value is
/// returned by [Context::cyclic] to short-circuit recursive call that would lead
/// to an infinite recursion.
pub fn op<A, R>(op: impl FnOnce(&mut Context<A, R>, A) -> R, args: A, pending: R) -> R {
    op(&mut Context::new(pending), args)
}

/// Operation context holds state of a cyclic operation preventing infinite
/// recursion.
#[derive(Debug)]
pub struct Context<A, R> {
    map: BTreeMap<A, R>,
    pending: R,
}

impl<A, R> Context<A, R> {
    fn new(pending: R) -> Self {
        Self {
            map: Default::default(),
            pending,
        }
    }
}

impl<A: std::cmp::Ord + Clone + std::fmt::Debug, R: Clone> Context<A, R> {
    pub fn cyclic(&mut self, op: impl FnOnce(&mut Context<A, R>, A) -> R, args: A) -> R {
        if let Some(v) = self.map.get(&args) {
            return v.to_owned();
        }

        self.map.insert(args.clone(), self.pending.clone());
        let r = op(self, args.clone());
        self.map.insert(args, r.clone());
        r
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

    fn non_recusive_eq(
        _ctx: &mut Context<(NonRecursiveT, NonRecursiveT), bool>,
        (lhs, rhs): (NonRecursiveT, NonRecursiveT),
    ) -> bool {
        lhs == rhs
    }

    #[test]
    fn recursive_eq_returns_true_for_equal_non_recursive_types() {
        let v1 = NonRecursiveT { b: true };
        let v2 = NonRecursiveT { b: true };

        assert!(super::op(non_recusive_eq, (v1, v2), true));
    }

    #[test]
    fn recursive_eq_returns_false_for_not_equal_non_recursive_types() {
        let v1 = NonRecursiveT { b: true };
        let v2 = NonRecursiveT { b: false };

        assert!(!super::op(non_recusive_eq, (v1, v2), true));
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct RecursiveT {
        i: usize,
        rec_ref: Option<Ref<Self>>,
    }

    fn recursive_eq(
        ctx: &mut super::Context<(Ref<RecursiveT>, Ref<RecursiveT>), bool>,
        (lhs, rhs): (Ref<RecursiveT>, Ref<RecursiveT>),
    ) -> bool {
        if lhs.i != rhs.i {
            false
        } else {
            match (lhs.rec_ref.clone(), rhs.rec_ref.clone()) {
                (Some(lhs), Some(rhs)) => ctx.cyclic(recursive_eq, (lhs.upgrade(), rhs.upgrade())),
                (None, None) => true,
                _ => false,
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

        assert!(super::op(recursive_eq, (v1.clone(), v2), true));
        assert!(super::op(recursive_eq, (v1.clone(), v1), true));
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

        assert!(!super::op(recursive_eq, (v1.clone(), v2), true));
    }
}
