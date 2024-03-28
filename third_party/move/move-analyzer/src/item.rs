// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0
use enum_iterator::Sequence;

#[derive(Clone)]
pub struct ItemStruct {}

impl std::fmt::Display for ItemStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

#[derive(Clone)]
pub enum Item {
    Const(ItemConst),
    Struct(ItemStruct),
    StructNameRef(ItemStructNameRef),
    Fun(ItemFun),
    MoveBuildInFun(MoveBuildInFun),
    SpecBuildInFun(SpecBuildInFun),
    SpecConst(ItemConst),
    ModuleName(ItemModuleName),
    Use(Vec<ItemUse>),
    Dummy,
}

#[derive(Clone)]
pub enum ItemUse {
    Module(ItemUseModule),
    Item(ItemUseItem),
}

#[derive(Clone)]
pub struct ItemUseModule {
    #[allow(dead_code)]
    pub(crate) is_test: bool,
}

#[derive(Clone)]
pub struct ItemUseItem {
    #[allow(dead_code)]
    pub(crate) is_test: bool,
}

#[derive(Clone)]
pub struct ItemModuleName {}

#[derive(Clone)]
pub struct ItemStructNameRef {}

#[derive(Clone)]
pub struct ItemFun {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttrTest {
    No,
    Test,
    TestOnly,
}

impl Default for Item {
    fn default() -> Self {
        Self::Dummy
    }
}

#[derive(Clone)]
pub struct ItemConst {}

#[derive(Clone)]
pub enum Access {
    AccessFiled(AccessFiled),
    KeyWords(&'static str),
}

#[derive(Clone)]
pub struct AccessFiled {}

#[derive(Clone)]
pub enum ItemOrAccess {
    Item(Item),
    Access(Access),
}

impl From<ItemOrAccess> for Item {
    fn from(x: ItemOrAccess) -> Self {
        match x {
            ItemOrAccess::Item(x) => x,
            _ => unreachable!(),
        }
    }
}

impl From<ItemOrAccess> for Access {
    fn from(x: ItemOrAccess) -> Self {
        match x {
            ItemOrAccess::Access(x) => x,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Sequence)]
pub enum MoveBuildInFun {
    MoveTo,
    MoveFrom,
    BorrowGlobalMut,
    BorrowGlobal,
    Exits,
}

impl MoveBuildInFun {
    pub(crate) fn to_static_str(self) -> &'static str {
        match self {
            MoveBuildInFun::MoveTo => "move_to",
            MoveBuildInFun::MoveFrom => "move_from",
            MoveBuildInFun::BorrowGlobalMut => "borrow_global_mut",
            MoveBuildInFun::BorrowGlobal => "borrow_global",
            MoveBuildInFun::Exits => "exists",
        }
    }
}

impl std::fmt::Display for MoveBuildInFun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Sequence)]
pub enum SpecBuildInFun {
    Exists,
    Global,
    Len,
    Update,
    Vec,
    Concat,
    Contains,
    IndexOf,
    Range,
    InRange,
    UpdateField,
    Old,
    TRACE,
}

impl SpecBuildInFun {
    pub(crate) fn to_static_str(self) -> &'static str {
        match self {
            Self::Exists => "exists",
            Self::Global => "global",
            Self::Len => "len",
            Self::Update => "update",
            Self::Vec => "vec",
            Self::Concat => "concat",
            Self::Contains => "contains",
            Self::IndexOf => "index_of",
            Self::Range => "range",
            Self::InRange => "in_range",
            Self::UpdateField => "update_field",
            Self::Old => "old",
            Self::TRACE => "TRACE",
        }
    }
}

impl std::fmt::Display for SpecBuildInFun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}
