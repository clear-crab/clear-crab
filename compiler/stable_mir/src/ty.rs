use super::{
    mir::Safety,
    mir::{Body, Mutability},
    with, DefId, Error, Symbol,
};
use crate::crate_def::CrateDef;
use crate::mir::alloc::{read_target_int, read_target_uint, AllocId};
use crate::target::MachineInfo;
use crate::{Filename, Opaque};
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Range;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Ty(pub usize);

impl Debug for Ty {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ty").field("id", &self.0).field("kind", &self.kind()).finish()
    }
}

/// Constructors for `Ty`.
impl Ty {
    /// Create a new type from a given kind.
    ///
    /// Note that not all types may be supported at this point.
    fn from_rigid_kind(kind: RigidTy) -> Ty {
        with(|cx| cx.new_rigid_ty(kind))
    }

    /// Create a new array type.
    pub fn try_new_array(elem_ty: Ty, size: u64) -> Result<Ty, Error> {
        Ok(Ty::from_rigid_kind(RigidTy::Array(elem_ty, Const::try_from_target_usize(size)?)))
    }

    /// Create a new array type from Const length.
    pub fn new_array_with_const_len(elem_ty: Ty, len: Const) -> Ty {
        Ty::from_rigid_kind(RigidTy::Array(elem_ty, len))
    }

    /// Create a new pointer type.
    pub fn new_ptr(pointee_ty: Ty, mutability: Mutability) -> Ty {
        Ty::from_rigid_kind(RigidTy::RawPtr(pointee_ty, mutability))
    }

    /// Create a new reference type.
    pub fn new_ref(reg: Region, pointee_ty: Ty, mutability: Mutability) -> Ty {
        Ty::from_rigid_kind(RigidTy::Ref(reg, pointee_ty, mutability))
    }

    /// Create a new pointer type.
    pub fn new_tuple(tys: &[Ty]) -> Ty {
        Ty::from_rigid_kind(RigidTy::Tuple(Vec::from(tys)))
    }

    /// Create a new closure type.
    pub fn new_closure(def: ClosureDef, args: GenericArgs) -> Ty {
        Ty::from_rigid_kind(RigidTy::Closure(def, args))
    }

    /// Create a new coroutine type.
    pub fn new_coroutine(def: CoroutineDef, args: GenericArgs, mov: Movability) -> Ty {
        Ty::from_rigid_kind(RigidTy::Coroutine(def, args, mov))
    }

    /// Create a new box type that represents `Box<T>`, for the given inner type `T`.
    pub fn new_box(inner_ty: Ty) -> Ty {
        with(|cx| cx.new_box_ty(inner_ty))
    }

    /// Create a type representing `usize`.
    pub fn usize_ty() -> Ty {
        Ty::from_rigid_kind(RigidTy::Uint(UintTy::Usize))
    }

    /// Create a type representing `bool`.
    pub fn bool_ty() -> Ty {
        Ty::from_rigid_kind(RigidTy::Bool)
    }
}

impl Ty {
    pub fn kind(&self) -> TyKind {
        with(|context| context.ty_kind(*self))
    }
}

/// Represents a constant in MIR or from the Type system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Const {
    /// The constant kind.
    pub(crate) kind: ConstantKind,
    /// The constant type.
    pub(crate) ty: Ty,
    /// Used for internal tracking of the internal constant.
    pub id: ConstId,
}

impl Const {
    /// Build a constant. Note that this should only be used by the compiler.
    pub fn new(kind: ConstantKind, ty: Ty, id: ConstId) -> Const {
        Const { kind, ty, id }
    }

    /// Retrieve the constant kind.
    pub fn kind(&self) -> &ConstantKind {
        &self.kind
    }

    /// Get the constant type.
    pub fn ty(&self) -> Ty {
        self.ty
    }

    /// Creates an interned usize constant.
    fn try_from_target_usize(val: u64) -> Result<Self, Error> {
        with(|cx| cx.usize_to_const(val))
    }

    /// Try to evaluate to a target `usize`.
    pub fn eval_target_usize(&self) -> Result<u64, Error> {
        with(|cx| cx.eval_target_usize(self))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstId(pub usize);

type Ident = Opaque;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    pub kind: RegionKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegionKind {
    ReEarlyParam(EarlyParamRegion),
    ReBound(DebruijnIndex, BoundRegion),
    ReStatic,
    RePlaceholder(Placeholder<BoundRegion>),
    ReErased,
}

pub(crate) type DebruijnIndex = u32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarlyParamRegion {
    pub def_id: RegionDef,
    pub index: u32,
    pub name: Symbol,
}

pub(crate) type BoundVar = u32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundRegion {
    pub var: BoundVar,
    pub kind: BoundRegionKind,
}

pub(crate) type UniverseIndex = u32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placeholder<T> {
    pub universe: UniverseIndex,
    pub bound: T,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span(usize);

impl Debug for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Span")
            .field("id", &self.0)
            .field("repr", &with(|cx| cx.span_to_string(*self)))
            .finish()
    }
}

impl Span {
    /// Return filename for diagnostic purposes
    pub fn get_filename(&self) -> Filename {
        with(|c| c.get_filename(self))
    }

    /// Return lines that corespond to this `Span`
    pub fn get_lines(&self) -> LineInfo {
        with(|c| c.get_lines(self))
    }
}

#[derive(Clone, Copy, Debug)]
/// Information you get from `Span` in a struct form.
/// Line and col start from 1.
pub struct LineInfo {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TyKind {
    RigidTy(RigidTy),
    Alias(AliasKind, AliasTy),
    Param(ParamTy),
    Bound(usize, BoundTy),
}

impl TyKind {
    pub fn rigid(&self) -> Option<&RigidTy> {
        if let TyKind::RigidTy(inner) = self { Some(inner) } else { None }
    }

    pub fn is_unit(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::Tuple(data)) if data.is_empty())
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::Bool))
    }

    pub fn is_trait(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::Dynamic(_, _, DynKind::Dyn)))
    }

    pub fn is_enum(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::Adt(def, _)) if def.kind() == AdtKind::Enum)
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::Adt(def, _)) if def.kind() == AdtKind::Struct)
    }

    pub fn is_union(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::Adt(def, _)) if def.kind() == AdtKind::Union)
    }

    pub fn is_fn(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::FnDef(..)))
    }

    pub fn is_fn_ptr(&self) -> bool {
        matches!(self, TyKind::RigidTy(RigidTy::FnPtr(..)))
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            TyKind::RigidTy(
                RigidTy::Bool
                    | RigidTy::Char
                    | RigidTy::Int(_)
                    | RigidTy::Uint(_)
                    | RigidTy::Float(_)
            )
        )
    }

    pub fn trait_principal(&self) -> Option<Binder<ExistentialTraitRef>> {
        if let TyKind::RigidTy(RigidTy::Dynamic(predicates, _, _)) = self {
            if let Some(Binder { value: ExistentialPredicate::Trait(trait_ref), bound_vars }) =
                predicates.first()
            {
                Some(Binder { value: trait_ref.clone(), bound_vars: bound_vars.clone() })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Returns the type of `ty[i]` for builtin types.
    pub fn builtin_index(&self) -> Option<Ty> {
        match self.rigid()? {
            RigidTy::Array(ty, _) | RigidTy::Slice(ty) => Some(*ty),
            _ => None,
        }
    }

    /// Returns the type and mutability of `*ty` for builtin types.
    ///
    /// The parameter `explicit` indicates if this is an *explicit* dereference.
    /// Some types -- notably unsafe ptrs -- can only be dereferenced explicitly.
    pub fn builtin_deref(&self, explicit: bool) -> Option<TypeAndMut> {
        match self.rigid()? {
            RigidTy::Adt(def, args) if def.is_box() => {
                Some(TypeAndMut { ty: *args.0.first()?.ty()?, mutability: Mutability::Not })
            }
            RigidTy::Ref(_, ty, mutability) => {
                Some(TypeAndMut { ty: *ty, mutability: *mutability })
            }
            RigidTy::RawPtr(ty, mutability) if explicit => {
                Some(TypeAndMut { ty: *ty, mutability: *mutability })
            }
            _ => None,
        }
    }

    /// Get the function signature for function like types (Fn, FnPtr, Closure, Coroutine)
    /// FIXME(closure)
    pub fn fn_sig(&self) -> Option<PolyFnSig> {
        match self {
            TyKind::RigidTy(RigidTy::FnDef(def, args)) => Some(with(|cx| cx.fn_sig(*def, args))),
            TyKind::RigidTy(RigidTy::FnPtr(sig)) => Some(sig.clone()),
            _ => None,
        }
    }

    /// Get the discriminant type for this type.
    pub fn discriminant_ty(&self) -> Option<Ty> {
        self.rigid().map(|ty| with(|cx| cx.rigid_ty_discriminant_ty(ty)))
    }
}

pub struct TypeAndMut {
    pub ty: Ty,
    pub mutability: Mutability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RigidTy {
    Bool,
    Char,
    Int(IntTy),
    Uint(UintTy),
    Float(FloatTy),
    Adt(AdtDef, GenericArgs),
    Foreign(ForeignDef),
    Str,
    Array(Ty, Const),
    Slice(Ty),
    RawPtr(Ty, Mutability),
    Ref(Region, Ty, Mutability),
    FnDef(FnDef, GenericArgs),
    FnPtr(PolyFnSig),
    Closure(ClosureDef, GenericArgs),
    Coroutine(CoroutineDef, GenericArgs, Movability),
    Dynamic(Vec<Binder<ExistentialPredicate>>, Region, DynKind),
    Never,
    Tuple(Vec<Ty>),
    CoroutineWitness(CoroutineWitnessDef, GenericArgs),
}

impl RigidTy {
    /// Get the discriminant type for this type.
    pub fn discriminant_ty(&self) -> Ty {
        with(|cx| cx.rigid_ty_discriminant_ty(self))
    }
}

impl From<RigidTy> for TyKind {
    fn from(value: RigidTy) -> Self {
        TyKind::RigidTy(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntTy {
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl IntTy {
    pub fn num_bytes(self) -> usize {
        match self {
            IntTy::Isize => crate::target::MachineInfo::target_pointer_width().bytes().into(),
            IntTy::I8 => 1,
            IntTy::I16 => 2,
            IntTy::I32 => 4,
            IntTy::I64 => 8,
            IntTy::I128 => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UintTy {
    Usize,
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl UintTy {
    pub fn num_bytes(self) -> usize {
        match self {
            UintTy::Usize => crate::target::MachineInfo::target_pointer_width().bytes().into(),
            UintTy::U8 => 1,
            UintTy::U16 => 2,
            UintTy::U32 => 4,
            UintTy::U64 => 8,
            UintTy::U128 => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatTy {
    F32,
    F64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movability {
    Static,
    Movable,
}

crate_def! {
    /// Hold information about a ForeignItem in a crate.
    pub ForeignDef;
}

crate_def! {
    /// Hold information about a function definition in a crate.
    pub FnDef;
}

impl FnDef {
    // Get the function body if available.
    pub fn body(&self) -> Option<Body> {
        with(|ctx| ctx.has_body(self.0).then(|| ctx.mir_body(self.0)))
    }
}

crate_def! {
    pub ClosureDef;
}

crate_def! {
    pub CoroutineDef;
}

crate_def! {
    pub ParamDef;
}

crate_def! {
    pub BrNamedDef;
}

crate_def! {
    pub AdtDef;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AdtKind {
    Enum,
    Union,
    Struct,
}

impl AdtDef {
    pub fn kind(&self) -> AdtKind {
        with(|cx| cx.adt_kind(*self))
    }

    /// Retrieve the type of this Adt.
    pub fn ty(&self) -> Ty {
        with(|cx| cx.def_ty(self.0))
    }

    /// Retrieve the type of this Adt instantiating the type with the given arguments.
    ///
    /// This will assume the type can be instantiated with these arguments.
    pub fn ty_with_args(&self, args: &GenericArgs) -> Ty {
        with(|cx| cx.def_ty_with_args(self.0, args))
    }

    pub fn is_box(&self) -> bool {
        with(|cx| cx.adt_is_box(*self))
    }

    /// The number of variants in this ADT.
    pub fn num_variants(&self) -> usize {
        with(|cx| cx.adt_variants_len(*self))
    }

    /// Retrieve the variants in this ADT.
    pub fn variants(&self) -> Vec<VariantDef> {
        self.variants_iter().collect()
    }

    /// Iterate over the variants in this ADT.
    pub fn variants_iter(&self) -> impl Iterator<Item = VariantDef> + '_ {
        (0..self.num_variants())
            .map(|idx| VariantDef { idx: VariantIdx::to_val(idx), adt_def: *self })
    }

    pub fn variant(&self, idx: VariantIdx) -> Option<VariantDef> {
        (idx.to_index() < self.num_variants()).then_some(VariantDef { idx, adt_def: *self })
    }
}

/// Definition of a variant, which can be either a struct / union field or an enum variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariantDef {
    /// The variant index.
    ///
    /// ## Warning
    /// Do not access this field directly!
    pub idx: VariantIdx,
    /// The data type where this variant comes from.
    /// For now, we use this to retrieve information about the variant itself so we don't need to
    /// cache more information.
    ///
    /// ## Warning
    /// Do not access this field directly!
    pub adt_def: AdtDef,
}

impl VariantDef {
    pub fn name(&self) -> Symbol {
        with(|cx| cx.variant_name(*self))
    }

    /// Retrieve all the fields in this variant.
    // We expect user to cache this and use it directly since today it is expensive to generate all
    // fields name.
    pub fn fields(&self) -> Vec<FieldDef> {
        with(|cx| cx.variant_fields(*self))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDef {
    /// The field definition.
    ///
    /// ## Warning
    /// Do not access this field directly! This is public for the compiler to have access to it.
    pub def: DefId,

    /// The field name.
    pub name: Symbol,
}

impl FieldDef {
    /// Retrieve the type of this field instantiating the type with the given arguments.
    ///
    /// This will assume the type can be instantiated with these arguments.
    pub fn ty_with_args(&self, args: &GenericArgs) -> Ty {
        with(|cx| cx.def_ty_with_args(self.def, args))
    }

    /// Retrieve the type of this field.
    pub fn ty(&self) -> Ty {
        with(|cx| cx.def_ty(self.def))
    }
}

impl Display for AdtKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AdtKind::Enum => "enum",
            AdtKind::Union => "union",
            AdtKind::Struct => "struct",
        })
    }
}

impl AdtKind {
    pub fn is_enum(&self) -> bool {
        matches!(self, AdtKind::Enum)
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, AdtKind::Struct)
    }

    pub fn is_union(&self) -> bool {
        matches!(self, AdtKind::Union)
    }
}

crate_def! {
    pub AliasDef;
}

crate_def! {
    pub TraitDef;
}

crate_def! {
    pub GenericDef;
}

crate_def! {
    pub ConstDef;
}

crate_def! {
    pub ImplDef;
}

crate_def! {
    pub RegionDef;
}

crate_def! {
    pub CoroutineWitnessDef;
}

/// A list of generic arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericArgs(pub Vec<GenericArgKind>);

impl std::ops::Index<ParamTy> for GenericArgs {
    type Output = Ty;

    fn index(&self, index: ParamTy) -> &Self::Output {
        self.0[index.index as usize].expect_ty()
    }
}

impl std::ops::Index<ParamConst> for GenericArgs {
    type Output = Const;

    fn index(&self, index: ParamConst) -> &Self::Output {
        self.0[index.index as usize].expect_const()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenericArgKind {
    Lifetime(Region),
    Type(Ty),
    Const(Const),
}

impl GenericArgKind {
    /// Panic if this generic argument is not a type, otherwise
    /// return the type.
    #[track_caller]
    pub fn expect_ty(&self) -> &Ty {
        match self {
            GenericArgKind::Type(ty) => ty,
            _ => panic!("{self:?}"),
        }
    }

    /// Panic if this generic argument is not a const, otherwise
    /// return the const.
    #[track_caller]
    pub fn expect_const(&self) -> &Const {
        match self {
            GenericArgKind::Const(c) => c,
            _ => panic!("{self:?}"),
        }
    }

    /// Return the generic argument type if applicable, otherwise return `None`.
    pub fn ty(&self) -> Option<&Ty> {
        match self {
            GenericArgKind::Type(ty) => Some(ty),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TermKind {
    Type(Ty),
    Const(Const),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AliasKind {
    Projection,
    Inherent,
    Opaque,
    Weak,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AliasTy {
    pub def_id: AliasDef,
    pub args: GenericArgs,
}

pub type PolyFnSig = Binder<FnSig>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FnSig {
    pub inputs_and_output: Vec<Ty>,
    pub c_variadic: bool,
    pub unsafety: Safety,
    pub abi: Abi,
}

impl FnSig {
    pub fn output(&self) -> Ty {
        self.inputs_and_output[self.inputs_and_output.len() - 1]
    }

    pub fn inputs(&self) -> &[Ty] {
        &self.inputs_and_output[..self.inputs_and_output.len() - 1]
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Abi {
    Rust,
    C { unwind: bool },
    Cdecl { unwind: bool },
    Stdcall { unwind: bool },
    Fastcall { unwind: bool },
    Vectorcall { unwind: bool },
    Thiscall { unwind: bool },
    Aapcs { unwind: bool },
    Win64 { unwind: bool },
    SysV64 { unwind: bool },
    PtxKernel,
    Msp430Interrupt,
    X86Interrupt,
    AmdGpuKernel,
    EfiApi,
    AvrInterrupt,
    AvrNonBlockingInterrupt,
    CCmseNonSecureCall,
    Wasm,
    System { unwind: bool },
    RustIntrinsic,
    RustCall,
    PlatformIntrinsic,
    Unadjusted,
    RustCold,
    RiscvInterruptM,
    RiscvInterruptS,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binder<T> {
    pub value: T,
    pub bound_vars: Vec<BoundVariableKind>,
}

impl<T> Binder<T> {
    pub fn skip_binder(self) -> T {
        self.value
    }

    pub fn map_bound_ref<F, U>(&self, f: F) -> Binder<U>
    where
        F: FnOnce(&T) -> U,
    {
        let Binder { value, bound_vars } = self;
        let new_value = f(value);
        Binder { value: new_value, bound_vars: bound_vars.clone() }
    }

    pub fn map_bound<F, U>(self, f: F) -> Binder<U>
    where
        F: FnOnce(T) -> U,
    {
        let Binder { value, bound_vars } = self;
        let new_value = f(value);
        Binder { value: new_value, bound_vars }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EarlyBinder<T> {
    pub value: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundVariableKind {
    Ty(BoundTyKind),
    Region(BoundRegionKind),
    Const,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BoundTyKind {
    Anon,
    Param(ParamDef, String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundRegionKind {
    BrAnon,
    BrNamed(BrNamedDef, String),
    BrEnv,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DynKind {
    Dyn,
    DynStar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExistentialPredicate {
    Trait(ExistentialTraitRef),
    Projection(ExistentialProjection),
    AutoTrait(TraitDef),
}

/// An existential reference to a trait where `Self` is not included.
///
/// The `generic_args` will include any other known argument.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistentialTraitRef {
    pub def_id: TraitDef,
    pub generic_args: GenericArgs,
}

impl Binder<ExistentialTraitRef> {
    pub fn with_self_ty(&self, self_ty: Ty) -> Binder<TraitRef> {
        self.map_bound_ref(|trait_ref| trait_ref.with_self_ty(self_ty))
    }
}

impl ExistentialTraitRef {
    pub fn with_self_ty(&self, self_ty: Ty) -> TraitRef {
        TraitRef::new(self.def_id, self_ty, &self.generic_args)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistentialProjection {
    pub def_id: TraitDef,
    pub generic_args: GenericArgs,
    pub term: TermKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamTy {
    pub index: u32,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundTy {
    pub var: usize,
    pub kind: BoundTyKind,
}

pub type Bytes = Vec<Option<u8>>;
pub type Size = usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Prov(pub AllocId);
pub type Align = u64;
pub type Promoted = u32;
pub type InitMaskMaterialized = Vec<u64>;

/// Stores the provenance information of pointers stored in memory.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ProvenanceMap {
    /// Provenance in this map applies from the given offset for an entire pointer-size worth of
    /// bytes. Two entries in this map are always at least a pointer size apart.
    pub ptrs: Vec<(Size, Prov)>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Allocation {
    pub bytes: Bytes,
    pub provenance: ProvenanceMap,
    pub align: Align,
    pub mutability: Mutability,
}

impl Allocation {
    /// Get a vector of bytes for an Allocation that has been fully initialized
    pub fn raw_bytes(&self) -> Result<Vec<u8>, Error> {
        self.bytes
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| error!("Found uninitialized bytes: `{:?}`", self.bytes))
    }

    /// Read a uint value from the specified range.
    pub fn read_partial_uint(&self, range: Range<usize>) -> Result<u128, Error> {
        if range.end - range.start > 16 {
            return Err(error!("Allocation is bigger than largest integer"));
        }
        if range.end > self.bytes.len() {
            return Err(error!(
                "Range is out of bounds. Allocation length is `{}`, but requested range `{:?}`",
                self.bytes.len(),
                range
            ));
        }
        let raw = self.bytes[range]
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| error!("Found uninitialized bytes: `{:?}`", self.bytes))?;
        read_target_uint(&raw)
    }

    /// Read this allocation and try to convert it to an unassigned integer.
    pub fn read_uint(&self) -> Result<u128, Error> {
        if self.bytes.len() > 16 {
            return Err(error!("Allocation is bigger than largest integer"));
        }
        let raw = self.raw_bytes()?;
        read_target_uint(&raw)
    }

    /// Read this allocation and try to convert it to a signed integer.
    pub fn read_int(&self) -> Result<i128, Error> {
        if self.bytes.len() > 16 {
            return Err(error!("Allocation is bigger than largest integer"));
        }
        let raw = self.raw_bytes()?;
        read_target_int(&raw)
    }

    /// Read this allocation and try to convert it to a boolean.
    pub fn read_bool(&self) -> Result<bool, Error> {
        match self.read_int()? {
            0 => Ok(false),
            1 => Ok(true),
            val @ _ => Err(error!("Unexpected value for bool: `{val}`")),
        }
    }

    /// Read this allocation as a pointer and return whether it represents a `null` pointer.
    pub fn is_null(&self) -> Result<bool, Error> {
        let len = self.bytes.len();
        let ptr_len = MachineInfo::target_pointer_width().bytes();
        if len != ptr_len {
            return Err(error!("Expected width of pointer (`{ptr_len}`), but found: `{len}`"));
        }
        Ok(self.read_uint()? == 0 && self.provenance.ptrs.is_empty())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstantKind {
    Allocated(Allocation),
    Unevaluated(UnevaluatedConst),
    Param(ParamConst),
    /// Store ZST constants.
    /// We have to special handle these constants since its type might be generic.
    ZeroSized,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamConst {
    pub index: u32,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnevaluatedConst {
    pub def: ConstDef,
    pub args: GenericArgs,
    pub promoted: Option<Promoted>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraitSpecializationKind {
    None,
    Marker,
    AlwaysApplicable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitDecl {
    pub def_id: TraitDef,
    pub unsafety: Safety,
    pub paren_sugar: bool,
    pub has_auto_impl: bool,
    pub is_marker: bool,
    pub is_coinductive: bool,
    pub skip_array_during_method_dispatch: bool,
    pub specialization_kind: TraitSpecializationKind,
    pub must_implement_one_of: Option<Vec<Ident>>,
    pub implement_via_object: bool,
    pub deny_explicit_impl: bool,
}

impl TraitDecl {
    pub fn generics_of(&self) -> Generics {
        with(|cx| cx.generics_of(self.def_id.0))
    }

    pub fn predicates_of(&self) -> GenericPredicates {
        with(|cx| cx.predicates_of(self.def_id.0))
    }

    pub fn explicit_predicates_of(&self) -> GenericPredicates {
        with(|cx| cx.explicit_predicates_of(self.def_id.0))
    }
}

pub type ImplTrait = EarlyBinder<TraitRef>;

/// A complete reference to a trait, i.e., one where `Self` is known.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitRef {
    pub def_id: TraitDef,
    /// The generic arguments for this definition.
    /// The first element must always be type, and it represents `Self`.
    args: GenericArgs,
}

impl TraitRef {
    pub fn new(def_id: TraitDef, self_ty: Ty, gen_args: &GenericArgs) -> TraitRef {
        let mut args = vec![GenericArgKind::Type(self_ty)];
        args.extend_from_slice(&gen_args.0);
        TraitRef { def_id, args: GenericArgs(args) }
    }

    pub fn try_new(def_id: TraitDef, args: GenericArgs) -> Result<TraitRef, ()> {
        match &args.0[..] {
            [GenericArgKind::Type(_), ..] => Ok(TraitRef { def_id, args }),
            _ => Err(()),
        }
    }

    pub fn args(&self) -> &GenericArgs {
        &self.args
    }

    pub fn self_ty(&self) -> Ty {
        let GenericArgKind::Type(self_ty) = self.args.0[0] else {
            panic!("Self must be a type, but found: {:?}", self.args.0[0])
        };
        self_ty
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Generics {
    pub parent: Option<GenericDef>,
    pub parent_count: usize,
    pub params: Vec<GenericParamDef>,
    pub param_def_id_to_index: Vec<(GenericDef, u32)>,
    pub has_self: bool,
    pub has_late_bound_regions: Option<Span>,
    pub host_effect_index: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenericParamDefKind {
    Lifetime,
    Type { has_default: bool, synthetic: bool },
    Const { has_default: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericParamDef {
    pub name: super::Symbol,
    pub def_id: GenericDef,
    pub index: u32,
    pub pure_wrt_drop: bool,
    pub kind: GenericParamDefKind,
}

pub struct GenericPredicates {
    pub parent: Option<TraitDef>,
    pub predicates: Vec<(PredicateKind, Span)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredicateKind {
    Clause(ClauseKind),
    ObjectSafe(TraitDef),
    SubType(SubtypePredicate),
    Coerce(CoercePredicate),
    ConstEquate(Const, Const),
    Ambiguous,
    AliasRelate(TermKind, TermKind, AliasRelationDirection),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClauseKind {
    Trait(TraitPredicate),
    RegionOutlives(RegionOutlivesPredicate),
    TypeOutlives(TypeOutlivesPredicate),
    Projection(ProjectionPredicate),
    ConstArgHasType(Const, Ty),
    WellFormed(GenericArgKind),
    ConstEvaluatable(Const),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosureKind {
    Fn,
    FnMut,
    FnOnce,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubtypePredicate {
    pub a: Ty,
    pub b: Ty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoercePredicate {
    pub a: Ty,
    pub b: Ty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AliasRelationDirection {
    Equate,
    Subtype,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitPredicate {
    pub trait_ref: TraitRef,
    pub polarity: ImplPolarity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutlivesPredicate<A, B>(pub A, pub B);

pub type RegionOutlivesPredicate = OutlivesPredicate<Region, Region>;
pub type TypeOutlivesPredicate = OutlivesPredicate<Ty, Region>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionPredicate {
    pub projection_ty: AliasTy,
    pub term: TermKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImplPolarity {
    Positive,
    Negative,
    Reservation,
}

pub trait IndexedVal {
    fn to_val(index: usize) -> Self;

    fn to_index(&self) -> usize;
}

macro_rules! index_impl {
    ($name:ident) => {
        impl IndexedVal for $name {
            fn to_val(index: usize) -> Self {
                $name(index)
            }
            fn to_index(&self) -> usize {
                self.0
            }
        }
    };
}

index_impl!(ConstId);
index_impl!(Ty);
index_impl!(Span);

/// The source-order index of a variant in a type.
///
/// For example, in the following types,
/// ```ignore(illustrative)
/// enum Demo1 {
///    Variant0 { a: bool, b: i32 },
///    Variant1 { c: u8, d: u64 },
/// }
/// struct Demo2 { e: u8, f: u16, g: u8 }
/// ```
/// `a` is in the variant with the `VariantIdx` of `0`,
/// `c` is in the variant with the `VariantIdx` of `1`, and
/// `g` is in the variant with the `VariantIdx` of `0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariantIdx(usize);

index_impl!(VariantIdx);
