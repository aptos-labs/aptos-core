import LeanerLang

namespace LeanerLang.Tests.VerificationLoops

leaner module 0x44::verification_loops where
  public fun count_to(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
    else break
    spec do
      invariant current <= limit
    return current

  spec count_to where
    ensures result == limit
    aborts_if false

  verify count_to

  public fun count_to_with_continue(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
      continue
    else break
    spec do
      invariant current <= limit
    return current

  spec count_to_with_continue where
    ensures result == limit
    aborts_if false

  verify count_to_with_continue

#leaner_require_native 0x44::verification_loops::count_to
#leaner_require_native 0x44::verification_loops::count_to_with_continue

end LeanerLang.Tests.VerificationLoops
