# Razen Compiler Rules

## Formal Memory, Ownership, and Safety Guarantees

*(Developer-Facing Specification)*

---

## 1. Purpose of This Document

This document defines the **observable compiler rules and guarantees** enforced by the Razen compiler.

* It explains **what the compiler guarantees**
* It does **not** expose internal algorithms
* It defines **what developers can rely on**
* It intentionally omits complex internal mechanics

> For conceptual design, motivation, and comparisons, see **DASO.md**.

---

## 2. Scope and Non-Goals

### In Scope

* Memory safety rules
* Ownership and sharing rules
* Deterministic lifetime guarantees
* Compile-time enforcement behavior

### Out of Scope

* Internal borrow graphs
* Lifetime constraint solving
* SSA dominance trees
* Escape graph algorithms
* Optimizer and backend behavior

These exist, but are **compiler responsibilities**.

---

## 3. Core Safety Contract

For all **safe Razen code**, the compiler guarantees:

1. No use-after-free
2. No double-free
3. No dangling references
4. No memory leaks
5. No undefined behavior from memory access
6. No data races outside `unsafe`
7. Deterministic memory release

If a program compiles, these guarantees **hold by definition**.

---

## 4. Memory Allocation Rules

### Rule M1 — Implicit Allocation

All memory allocations default to the **current DASO region**.

* No heap allocation keyword is required
* Allocation strategy is selected by the compiler

---

### Rule M2 — Region Lifetime

A DASO region:

* Is created at `act` entry
* Is destroyed at `act` exit
* Has a strictly lexical lifetime

Region destruction is deterministic and unconditional.

---

### Rule M3 — Region Deallocation

* Region deallocation is **O(1)**
* No object-level destructors are required for memory
* Resource cleanup is handled separately (`defer`)

---

## 5. Escape and Promotion Rules

### Rule E1 — Escape Detection

A value **escapes** its region if it:

* Is returned from an `act`
* Is stored in a longer-lived structure
* Is captured by an async or forked task
* Is assigned to a shared binding

---

### Rule E2 — Automatic Promotion

When a value escapes:

* The compiler automatically promotes it to heap-backed storage
* Ownership is assigned without developer syntax
* The original region remains valid

This process is **implicit and guaranteed**.

---

## 6. Ownership Rules (DASO Ownership)

### Rule O1 — Single Ownership

Every value has exactly one owner unless explicitly shared.

---

### Rule O2 — Move Semantics

* Assignments transfer ownership by default
* After a move, the previous binding is invalid

Violations are compile-time errors.

---

### Rule O3 — Borrow Inference

The compiler may infer a temporary borrow if:

* The value is read-only
* The value does not escape
* The callee does not store the value

Borrowing is invisible in syntax.

---

## 7. Shared Ownership Rules

### Rule S1 — Explicit Sharing

Shared ownership must be explicitly declared using `shared`.

If `shared` is not present:

* Multiple owners are forbidden
* The compiler rejects ambiguous ownership

---

### Rule S2 — Reference Counting

Shared values use reference counting.

* Non-atomic by default
* Atomic when crossing concurrency boundaries

The choice is compiler-controlled.

---

### Rule S3 — Cycle Handling

* Cycles are allowed only in shared graphs
* Cycle handling is implementation-defined
* The compiler may warn about unbounded shared cycles

---

## 8. Concurrency Interaction Rules

### Rule C1 — Thread Safety

* Non-shared values cannot cross `async` or `fork`
* Shared values may cross concurrency boundaries

---

### Rule C2 — Data Race Prevention

* Mutable access to shared data is restricted
* Unsafe concurrent mutation requires `unsafe`

The compiler enforces race freedom by default.

---

## 9. `defer` and Resource Rules

### Rule D1 — Deterministic Execution

* `defer` statements execute at scope exit
* Execution order is reverse-declaration order

---

### Rule D2 — Resource Separation

* `defer` is for **resources**, not memory
* Memory is released by DASO regions

---

## 10. Unsafe Code Boundary Rules

### Rule U1 — Explicit Opt-In

Unsafe behavior requires an `unsafe` block.

---

### Rule U2 — Safety Containment

* Undefined behavior inside `unsafe` does not affect safe code
* Unsafe blocks are statically isolated

---

### Rule U3 — Zero Overhead

The compiler inserts **no checks** inside `unsafe`.

---

## 11. Compile-Time Error Model

The compiler rejects programs that violate:

* Ownership uniqueness
* Lifetime validity
* Illegal sharing
* Invalid escapes
* Concurrency safety

Errors are **deterministic and reproducible**.

---

## 12. What Developers Do NOT Need to Know

The following are intentionally hidden:

* Lifetime graphs
* Borrow regions
* Promotion heuristics
* Allocation tier selection
* ARC optimization strategies

These are compiler implementation details.

---

## 13. Relationship to DASO.md

This document defines **rules and guarantees**.
**DASO.md** defines **design philosophy, motivation, and comparison**.

For deeper understanding:

> **See DASO.md**

---

## 14. Final Statement

Razen’s compiler rules exist to ensure that:

* Developers write simple code
* The compiler performs complex reasoning
* Performance remains predictable
* Safety is never optional

This separation is intentional.

---

**Razen Compiler Rule Philosophy:**
*The compiler does the hard work so developers do not have to.*