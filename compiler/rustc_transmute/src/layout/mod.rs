use std::fmt::{self, Debug};
use std::hash::Hash;

pub(crate) mod tree;
pub(crate) use tree::Tree;

pub(crate) mod nfa;
pub(crate) use nfa::Nfa;

pub(crate) mod dfa;
pub(crate) use dfa::Dfa;

#[derive(Debug)]
pub(crate) struct Uninhabited;

/// An instance of a byte is either initialized to a particular value, or uninitialized.
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) enum Byte {
    Uninit,
    Init(u8),
}

impl fmt::Debug for Byte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Uninit => f.write_str("??u8"),
            Self::Init(b) => write!(f, "{b:#04x}u8"),
        }
    }
}

pub(crate) trait Def: Debug + Hash + Eq + PartialEq + Copy + Clone {
    fn has_safety_invariants(&self) -> bool;
}
pub trait Ref: Debug + Hash + Eq + PartialEq + Copy + Clone {
    fn min_align(&self) -> usize;

    fn is_mutable(&self) -> bool;
}

impl Def for ! {
    fn has_safety_invariants(&self) -> bool {
        unreachable!()
    }
}

impl Ref for ! {
    fn min_align(&self) -> usize {
        unreachable!()
    }
    fn is_mutable(&self) -> bool {
        unreachable!()
    }
}

#[cfg(feature = "rustc")]
pub mod rustc {
    use rustc_middle::mir::Mutability;
    use rustc_middle::ty::{self, Ty};

    /// A reference in the layout.
    #[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
    pub struct Ref<'tcx> {
        pub lifetime: ty::Region<'tcx>,
        pub ty: Ty<'tcx>,
        pub mutability: Mutability,
        pub align: usize,
    }

    impl<'tcx> super::Ref for Ref<'tcx> {
        fn min_align(&self) -> usize {
            self.align
        }

        fn is_mutable(&self) -> bool {
            match self.mutability {
                Mutability::Mut => true,
                Mutability::Not => false,
            }
        }
    }
    impl<'tcx> Ref<'tcx> {}

    /// A visibility node in the layout.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub enum Def<'tcx> {
        Adt(ty::AdtDef<'tcx>),
        Variant(&'tcx ty::VariantDef),
        Field(&'tcx ty::FieldDef),
        Primitive,
    }

    impl<'tcx> super::Def for Def<'tcx> {
        fn has_safety_invariants(&self) -> bool {
            // Rust presently has no notion of 'unsafe fields', so for now we
            // make the conservative assumption that everything besides
            // primitive types carry safety invariants.
            self != &Self::Primitive
        }
    }
}
