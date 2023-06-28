// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Most or all copied from Rust, which is Apache-2.0 OR MIT

#include "llvm/IR/Instructions.h"

using namespace llvm;
using namespace llvm::sys;

enum LLVMRustAttribute {
  AlwaysInline = 0,
  ByVal = 1,
  Cold = 2,
  InlineHint = 3,
  MinSize = 4,
  Naked = 5,
  NoAlias = 6,
  NoCapture = 7,
  NoInline = 8,
  NonNull = 9,
  NoRedZone = 10,
  NoReturn = 11,
  NoUnwind = 12,
  OptimizeForSize = 13,
  ReadOnly = 14,
  SExt = 15,
  StructRet = 16,
  UWTable = 17,
  ZExt = 18,
  InReg = 19,
  SanitizeThread = 20,
  SanitizeAddress = 21,
  SanitizeMemory = 22,
  NonLazyBind = 23,
  OptimizeNone = 24,
  ReturnsTwice = 25,
  ReadNone = 26,
  InaccessibleMemOnly = 27,
  SanitizeHWAddress = 28,
  WillReturn = 29,
  StackProtectReq = 30,
  StackProtectStrong = 31,
  StackProtect = 32,
  NoUndef = 33,
  SanitizeMemTag = 34,
};

static Attribute::AttrKind fromRust(LLVMRustAttribute Kind) {
  switch (Kind) {
  case AlwaysInline:
    return Attribute::AlwaysInline;
  case ByVal:
    return Attribute::ByVal;
  case Cold:
    return Attribute::Cold;
  case InlineHint:
    return Attribute::InlineHint;
  case MinSize:
    return Attribute::MinSize;
  case Naked:
    return Attribute::Naked;
  case NoAlias:
    return Attribute::NoAlias;
  case NoCapture:
    return Attribute::NoCapture;
  case NoInline:
    return Attribute::NoInline;
  case NonNull:
    return Attribute::NonNull;
  case NoRedZone:
    return Attribute::NoRedZone;
  case NoReturn:
    return Attribute::NoReturn;
  case NoUnwind:
    return Attribute::NoUnwind;
  case OptimizeForSize:
    return Attribute::OptimizeForSize;
  case ReadOnly:
    return Attribute::ReadOnly;
  case SExt:
    return Attribute::SExt;
  case StructRet:
    return Attribute::StructRet;
  case UWTable:
    return Attribute::UWTable;
  case ZExt:
    return Attribute::ZExt;
  case InReg:
    return Attribute::InReg;
  case SanitizeThread:
    return Attribute::SanitizeThread;
  case SanitizeAddress:
    return Attribute::SanitizeAddress;
  case SanitizeMemory:
    return Attribute::SanitizeMemory;
  case NonLazyBind:
    return Attribute::NonLazyBind;
  case OptimizeNone:
    return Attribute::OptimizeNone;
  case ReturnsTwice:
    return Attribute::ReturnsTwice;
  case ReadNone:
    return Attribute::ReadNone;
  case InaccessibleMemOnly:
    return Attribute::InaccessibleMemOnly;
  case SanitizeHWAddress:
    return Attribute::SanitizeHWAddress;
  case WillReturn:
    return Attribute::WillReturn;
  case StackProtectReq:
    return Attribute::StackProtectReq;
  case StackProtectStrong:
    return Attribute::StackProtectStrong;
  case StackProtect:
    return Attribute::StackProtect;
  case NoUndef:
    return Attribute::NoUndef;
  case SanitizeMemTag:
    return Attribute::SanitizeMemTag;
  }
  report_fatal_error("bad AttributeKind");
}

template<typename T> static inline void AddAttributes(T *t, unsigned Index,
                                                      LLVMAttributeRef *Attrs, size_t AttrsLen) {
  AttributeList PAL = t->getAttributes();
  AttributeList PALNew;
  AttrBuilder B(t->getContext());
  for (LLVMAttributeRef Attr : makeArrayRef(Attrs, AttrsLen))
    B.addAttribute(unwrap(Attr));
  PALNew = PAL.addAttributesAtIndex(t->getContext(), Index, B);
  t->setAttributes(PALNew);
}

extern "C" void LLVMRustAddFunctionAttributes(LLVMValueRef Fn, unsigned Index,
                                              LLVMAttributeRef *Attrs, size_t AttrsLen) {
  Function *F = unwrap<Function>(Fn);
  AddAttributes(F, Index, Attrs, AttrsLen);
}

extern "C" void LLVMRustAddCallSiteAttributes(LLVMValueRef Instr, unsigned Index,
                                              LLVMAttributeRef *Attrs, size_t AttrsLen) {
  CallBase *Call = unwrap<CallBase>(Instr);
  AddAttributes(Call, Index, Attrs, AttrsLen);
}

extern "C" LLVMAttributeRef LLVMRustCreateAttrNoValue(LLVMContextRef C,
                                                      LLVMRustAttribute RustAttr) {
  return wrap(Attribute::get(*unwrap(C), fromRust(RustAttr)));
}
