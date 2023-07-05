use super::with;

#[derive(Copy, Clone, Debug)]
pub struct Ty(pub usize);

impl Ty {
    pub fn kind(&self) -> TyKind {
        with(|context| context.ty_kind(*self))
    }
}

#[derive(Clone, Debug)]
pub enum TyKind {
    RigidTy(RigidTy),
}

#[derive(Clone, Debug)]
pub enum RigidTy {
    Bool,
    Tuple(Vec<Ty>),
}
