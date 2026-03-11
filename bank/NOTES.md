# Ownership - Rust way to solve unexpected updates!

1. Every value is 'owned' by a single variable, argument, struct, vector, etc at a time
2. Reassigning the value to a variable, passing it to a function, putting it into a vector, etc, moves the value. The old owner can't be used to access the value anymore!


# Borrowing.
1. You can create many read-only references to a value. These refs can exist at a same time.
2. !! You can't move a value while a reference to the value exists.
3. You can make a writeable (MUTABLE) reference to a value only if there are no READ-ONLY references currently in use. ONLY 1 mutable reference can exist at a time.
4. Mutable references allow us to read or change a value without MOVING it.
5. !! You can't mutate a value through the owner when any ref (mutable or immutable) to the value exists.
6. Some TYPES of values are COPIED instead of MOVED!

# Lifetimes - How long an owner/reference exists, before RUST decide to reclaim the memory.
1. When an owner goes out of scope, the value owned by it is DROPPED - meaning the memory is cleaned.
2. !! There can't be references to a value when its owner goes out of scope.
3. References to a value can't outlive the value they refer to.