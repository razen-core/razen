# Razen Language Specification: Keywords

This document is the official reference for all Razen keywords.  
**Philosophy:** Razen combines the simplicity of Go, the safety of Rust, and the power of Python for AI/ML.  
**Total Keywords:** ~35 (complete language).

---

## 1. Variables & Constants

- `mut`    – Declare a mutable variable. Variables are immutable by default.
- `const`  – Declare a compile-time constant value.
- `shared` – Declare a reference-counted shared value (opt-in shared ownership).

**Declaration syntax — `name: Type`**

The colon is the universal separator between a name and its type.

```razen
// Immutable — inferred
user_id := 1
ratio   := 0.75
lang    := "Razen"

// Immutable — explicit type
count:    int   := 42
name:     str   := "Razen"
price:    float := 99.99
is_ready: bool  := true

// Explicit sized types when width matters
pixel:   u8  := 255u8
weight:  f32 := 0.5f32
port:    u16 := 8080u16
delta:   i32 := -1000i32

// Mutable — always requires explicit type
mut score:    int   = 0
mut username: str   = "guest"
mut cursor:   uint  = 0u

// Constants — always require explicit type
const MAX_SCORE: int = 100
const APP_NAME:  str = "Razen Lang"
const PI:        f64 = 3.14159265358979

// Shared
shared cache: map[str, str] = map[]
```

**Mutability rules:**
- `:=` — immutable binding, type inferred or explicit
- `mut name: Type = value` — mutable binding, type always explicit
- `const NAME: Type = value` — compile-time constant

**Compound assignment (mutable variables only):**

```razen
mut score: int = 10

score += 5     // score = score + 5  → 15
score -= 3     // score = score - 3  → 12
score *= 2     // score = score * 2  → 24
score /= 4     // score = score / 4  → 6
score %= 4     // score = score % 4  → 2
score **= 2    // score = score ** 2 → 4

mut ratio: float = 1.0
ratio += 0.5   // 1.5
ratio *= 2.0   // 3.0

mut tag: str = "hello"
tag += " world"    // "hello world"
```

---

## 2. Types & Data Structures

- `struct` – Define a custom data structure with named fields.
- `enum`   – Define a set of named variants or a tagged union.
- `trait`  – Define a shared behavior or interface.
- `impl`   – Implement a trait or method for a specific type.
- `alias`  – Create a shortcut name for a complex type.

**Struct — fields use `name: Type`**

```razen
struct Point {
    x: float,
    y: float,
}

struct User {
    id:        int,
    name:      str,
    email:     str,
    is_active: bool,
    score:     float,
}

// Instantiation
alice := User {
    id:        1,
    name:      "Alice",
    email:     "alice@razen.dev",
    is_active: true,
    score:     91.5,
}

// Struct update syntax — copy with specific overrides
renamed     := User { name: "Alicia", ..alice }
deactivated := User { is_active: false, ..alice }
updated     := User { name: "Alicia", score: 95.0, ..alice }
```

**Enum — positional and named field variants**

```razen
// Simple enum
enum Direction { North, South, East, West }

// Positional data
enum Shape {
    Circle(float),
    Rectangle(float, float),
    Point,
}

// Named field variants
enum Event {
    Click    { x: float, y: float },
    Resize   { width: int, height: int },
    KeyPress { key: char, shift: bool, ctrl: bool },
    Quit,
}

// Mixed positional and named
enum ApiError {
    NotFound,
    Timeout(int),
    ParseError { line: int, msg: str },
    Network { code: int, reason: str },
}
```

**Trait — with optional default implementations**

```razen
trait Describable {
    // Required — every implementor must define this
    act describe(self) str

    // Default — works out of the box, can be overridden
    act short_desc(self) str {
        desc := self.describe()
        if desc.len() > 50 { desc[..50] } else { desc }
    }

    act print_info(self) void {
        println(self.describe())
    }
}

trait Shape {
    act area(self) float
    act perimeter(self) float

    // Default derived from area
    act is_larger_than(self, other: Self) bool {
        self.area() > other.area()
    }
}
```

**Generic struct with trait bounds**

```razen
struct Box[T] {
    value: T,
    label: str,
}

struct Renderer[T: Shape] {
    target: T,
    color:  str,
}
```

**`where` clause — for complex generic constraints**

```razen
// Inline bounds — simple cases
act total_area[T: Shape](shapes: vec[T]) float { ... }

// where clause — long or multiple bounds
act process[T, U](shape: T, ctx: U) str
where
    T: Shape + Describable,
    U: Display,
{
    "{ctx.to_string()}: {shape.describe()}"
}

struct Pipeline[I, O]
where
    I: Clone + Eq,
    O: Display,
{
    stages: vec[str],
}

impl[T] Box[T]
where
    T: Clone + Display,
{
    act cloned_str(self) str {
        self.value.clone().to_string()
    }
}
```

**Alias**

```razen
alias UserId       = int
alias Score        = float
alias NameMap      = map[str, str]
alias Vector3      = [float; 3]
alias ApiResult[T] = result[T, str]
alias MaybeUser    = option[User]
```

---

## 3. Functions & Modules

- `act`  – Define a function or action. Creates a scoped memory region (DASO).
- `ret`  – Early return from an action.
- `use`  – Import modules or specific items.
- `pub`  – Mark an item as public. Supports visibility levels.

**Function syntax — parameters use `name: Type`**

```razen
// Standard
act greet(name: str) str {
    "Hello, {name}!"
}

// One-liner with ->
act square(n: int) int -> n * n

// Void
act log(msg: str) void { println(msg) }

// Tuple return
act div_mod(a: int, b: int) (int, int) {
    (a / b, a % b)
}

// Generic with bound
act total_area[T: Shape](shapes: vec[T]) float {
    shapes.reduce(0.0, |acc, s| acc + s.area())
}

// where clause
act render[T, C](shape: T, ctx: C) str
where
    T: Shape + Describable,
    C: Display,
{
    "{ctx.to_string()}: {shape.describe()}"
}

// never — function that never returns
act panic_with(msg: str) never {
    println("PANIC: {msg}")
    std.process.exit(1)
}
```

**`ret` — early return only. Last expression is always the implicit return.**

```razen
act parse_age(input: str) result[int, str] {
    age := input.parse_int()?
    if age < 0   { ret err("Age cannot be negative") }
    if age > 150 { ret err("Age too high") }
    ok(age)    // implicit return — no ret needed
}
```

**`pub` — visibility levels**

```razen
pub           // visible everywhere — any module
pub(pkg)      // visible within this package (folder tree)
pub(mod)      // visible to sibling files in the same folder
              // (no keyword) — private, current file only
```

```razen
pub struct User { ... }               // public
pub(pkg) struct Cache { ... }         // package-internal
pub(mod) act setup() void { ... }     // module-internal
act hash_password(p: str) str { ... } // private
```

---

## 4. Control Flow & Logic

- `if`    – Conditional branching. No parentheses required.
- `else`  – Alternative branch.
- `loop`  – Universal loop construct (ranges, collections, while-style, infinite).
- `break` – Exit a loop, optionally with a value.
- `next`  – Skip to the next iteration.
- `match` – Exhaustive pattern matching.
- `guard` – Early exit condition. Keeps code flat.
- `!`   – Logical NOT.

**`!` — logical negation**

```razen
// ! is the logical NOT operator
if !is_valid { ret err("invalid") }

is_inactive := !user.is_active
nothing_found := !results.is_some()

// Combining
if !(x > 0 && y > 0) {
    println("at least one non-positive")
}
```

**`if` — conditional**

```razen
if score > 90 {
    println("A")
} else if score > 70 {
    println("B")
} else {
    println("C")
}

// if as expression
grade := if score > 90 { "A" } else if score > 70 { "B" } else { "C" }
```

**`loop` — universal loop**

```razen
// Range — exclusive / inclusive
loop i in 0..10    { println(i) }
loop i in 0..=10   { println(i) }

// Collection
loop item in values { println(item) }
loop (i, val) in items.enumerate() { println("{i}: {val}") }

// While-style
mut n: int = 0
loop n < 100 { n += 1 }

// Infinite
loop {
    data := read_next()
    if data.is_empty() { break }
    process(data)
}

// loop as expression — break returns value
result: int := loop {
    val := compute()
    if val > 10 { break val }
}

// loop else — runs if loop exits without break
found: option[int] := loop i in 0..items.len() {
    if items[i] == target { break some(i) }
} else {
    none
}
```

**Labeled loops — target nested break/next**

```razen
'outer: loop i in 0..10 {
    loop j in 0..10 {
        if i * j > 50 { break 'outer }
    }
}

'rows: loop row in matrix {
    loop cell in row {
        if cell < 0 { next 'rows }
        process(cell)
    }
}

// break with value from labeled loop
coords: (int, int) := 'find: loop i in 0..grid.len() {
    loop j in 0..grid[i].len() {
        if grid[i][j] == target { break 'find (i, j) }
    }
} else {
    (-1, -1)
}
```

**`match` — exhaustive, must cover all cases or use `_`**

```razen
// Enum match
match direction {
    Direction.North -> println("north"),
    Direction.South -> println("south"),
    Direction.East  -> println("east"),
    Direction.West  -> println("west"),
}

// match as expression
area := match shape {
    Shape.Circle(r)       -> 3.14159 * r * r,
    Shape.Rectangle(w, h) -> w * h,
    Shape.Point           -> 0.0,
}

// Match guard
match score {
    n if n >= 90 -> "A",
    n if n >= 80 -> "B",
    n if n >= 70 -> "C",
    _            -> "F",
}

// Named enum variant
match event {
    Event.Click { x, y }           -> handle_click(x, y),
    Event.Resize { width, height } -> resize(width, height),
    Event.KeyPress { key, _ }      -> handle_key(key),
    Event.Quit                      -> {},
    _                               -> {},
}
```

**`if let` and `loop let`**

```razen
// if let — conditional bind
if let some(user) = find_user(id) {
    println(user.name)
}

if let Event.Click { x, y } = event {
    println("click at {x},{y}")
}

// loop let — loop while pattern matches
loop let some(task) = queue.dequeue() {
    process(task)
}
```

**`guard` — flat preconditions**

```razen
act process(id: int, name: str) result[str, str] {
    guard id > 0   else { ret err("id must be positive") }
    guard name != "" else { ret err("name is required") }
    ok("processed")
}
```

---

## 5. Pattern Binding — `_`

`_` matches any value and discards it. No binding is created.

```razen
// Catch-all in match
match status {
    Status.Active -> "active",
    _             -> "other",
}

// Ignore a position in destructuring
(first, _, third) := (1, 2, 3)
{ name, _ } := user

// Suppress unused warnings
_ := side_effect()

// Ignored parameter
act on_event(_: Event) void { println("fired") }
```

---

## 6. Error Handling & Resources

- `defer` – Schedule code to run at scope exit. Reverse declaration order.

```razen
act process_file(path: str) result[str, str] {
    f := File.open(path)?
    defer f.close()
    data := f.read()?
    ok(data)
}
```

---

## 7. Concurrency

- `async` – Define an asynchronous action.
- `await` – Wait for an async task.
- `fork`  – Spawn concurrent tasks and await all (structured concurrency).

```razen
async act fetch(url: str) result[str, str] {
    res := http.get(url).await?
    ok(res.body)
}

// fork — concurrent tasks, waits for all
(user_res, posts_res) := fork {
    load_user(1).await,
    load_posts(1).await,
}

// fork with named bindings
fork {
    user_data  <- load_user(1).await,
    post_data  <- load_posts(1).await,
}
// both user_data and post_data available here

// fork loop — spawn one task per item
results := fork loop id in user_ids {
    load_user(id).await
}
// results: vec[result[User, str]]
```

---

## 8. Memory Safety

- `unsafe` – Disable safety checks for a block.

```razen
unsafe {
    ptr  := &value
    data := *ptr
    buf  := alloc(1024)
    free(buf)
}
```

---

## 9. Attributes — `@`

Attributes attach metadata to declarations. Args use `[]`.

```razen
@test
act test_add() void { assert add(2, 3) == 5 }

@inline
act fast_add(a: int, b: int) int -> a + b

@derive[Debug, Clone, Eq]
struct Point { x: float, y: float }

@derive[Debug, Clone, Eq, Ord, Hash]
struct Version { major: int, minor: int, patch: int }

@json[rename_all: "camelCase"]
struct ApiRequest { user_id: int, full_name: str }

@deprecated["use new_parse instead"]
act old_parse(input: str) int { ... }

@extern["C"]
act c_malloc(size: uint) bytes

@cfg[target: "linux"]
act get_home() str { ... }

@allow[unused]
act prototype() void { }
```

**Derivable traits:**

| Trait     | What it gives you                        |
|-----------|------------------------------------------|
| `Debug`   | `.debug_str()` — formatted struct repr  |
| `Clone`   | `.clone()` — deep copy                  |
| `Eq`      | `==`, `!=`                              |
| `Ord`     | `<`, `>`, `<=`, `>=`, `.cmp()`         |
| `Display` | `.to_string()`                          |
| `Hash`    | `.hash()` — for use as a map key        |

---

## 10. Type & Operator Keywords

- `in`    – Membership or iteration keyword.
- `as`    – Explicit type cast.
- `is`    – Runtime type check.
- `self`  – Current instance.
- `Self`  – The type being implemented.

```razen
// as — type cast
score_f := score as float
byte_val := 300i32 as u8     // truncates
letter   := 65u8 as char     // 'A'

// is — type check
if value is int { println("integer") }

// self / Self
impl User {
    act greet(self) str { "Hello, {self.name}!" }
}

impl Builder {
    act new() Self { Builder { ... } }
    act name(mut self, n: str) Self { self.name = n; self }
}
```

---

## 11. Primitive Types Reference

| Type     | Width  | Default? | Notes                              |
|----------|--------|----------|------------------------------------|
| `bool`   | 1-bit  | —        | `true` / `false`                   |
| `int`    | 64-bit | ✓        | Default signed int (alias `i64`)   |
| `uint`   | 64-bit | ✓        | Default unsigned int (alias `u64`) |
| `float`  | 64-bit | ✓        | Default float (alias `f64`)        |
| `i8`     | 8-bit  | —        | Tiny signed                        |
| `i16`    | 16-bit | —        | Small signed                       |
| `i32`    | 32-bit | —        | Mid signed                         |
| `i64`    | 64-bit | —        | Explicit alias for `int`           |
| `u8`     | 8-bit  | —        | Bytes, pixels, ASCII               |
| `u16`    | 16-bit | —        | Ports, small counts                |
| `u32`    | 32-bit | —        | Common in C/system APIs            |
| `u64`    | 64-bit | —        | Explicit alias for `uint`          |
| `f32`    | 32-bit | —        | GPU, tensors, ML weights           |
| `f64`    | 64-bit | —        | Explicit alias for `float`         |
| `char`   | 32-bit | —        | Single Unicode scalar value        |
| `str`    | —      | —        | UTF-8 string                       |
| `bytes`  | —      | —        | Raw byte buffer                    |
| `void`   | —      | —        | No return value                    |
| `never`  | —      | —        | Function that never returns        |
| `tensor` | —      | —        | N-dimensional array (AI/ML)        |

---

## 12. Closures

```razen
// Typed params — name: Type
double := |x: int| x * 2
add    := |a: int, b: int| a + b

// Inferred (inline usage)
nums.map(|x| x * 2)
nums.filter(|x| x % 2 == 0)
nums.reduce(0, |acc, x| acc + x)

// Closure as parameter
act apply(value: int, f: |int| -> int) int { f(value) }

// Closure returning closure
act make_adder(n: int) |int| -> int { |x| x + n }
```

---

## 13. Naming Convention — Spine Case

**Rule: capitalize the last word (the noun/object).**

```razen
// Variables and functions — last word capitalized
user_Account
email_Address
process_Payment
get_user_by_Id
parse_Config_File    // File is the noun

// Types — PascalCase
User
NetworkError
ApiResult

// Constants — ALL_CAPS
MAX_SCORE
APP_NAME
PI
```

---

## Complete Keyword Table

| Keyword   | Category      | Purpose                                    |
|-----------|---------------|--------------------------------------------|
| `mut`     | Variable      | Mutable binding                            |
| `const`   | Variable      | Compile-time constant                      |
| `shared`  | Memory        | Reference-counted shared ownership         |
| `struct`  | Type          | Custom data structure                      |
| `enum`    | Type          | Sum type / tagged union                    |
| `trait`   | Type          | Interface / shared behavior                |
| `impl`    | Type          | Implement trait or methods                 |
| `alias`   | Type          | Type alias                                 |
| `act`     | Function      | Define a function                          |
| `ret`     | Function      | Early return                               |
| `use`     | Module        | Import a module or items                   |
| `pub`     | Module        | Export / visibility                        |
| `if`      | Control flow  | Conditional branch                         |
| `else`    | Control flow  | Alternative branch                         |
| `loop`    | Control flow  | Universal loop                             |
| `break`   | Control flow  | Exit loop, optionally with value           |
| `next`    | Control flow  | Skip to next iteration                     |
| `match`   | Control flow  | Exhaustive pattern match                   |
| `guard`   | Control flow  | Flat early-exit precondition               |
| `!`       | Logic         | Logical NOT                                |
| `_`       | Pattern       | Wildcard — discard and catch-all           |
| `in`      | Operator      | Membership / iteration                     |
| `as`      | Operator      | Type cast                                  |
| `is`      | Operator      | Type check                                 |
| `self`    | Method        | Current instance                           |
| `Self`    | Method        | Current type                               |
| `defer`   | Resource      | Scheduled scope-exit cleanup               |
| `async`   | Concurrency   | Async function                             |
| `await`   | Concurrency   | Wait for async result                      |
| `fork`    | Concurrency   | Structured concurrent tasks                |
| `unsafe`  | Memory        | Disable safety checks                      |
| `where`   | Generics      | Constraint clause for complex bounds       |
| `true`    | Literal       | Boolean true                               |
| `false`   | Literal       | Boolean false                              |

**See also:**
- `symbols.md` — operators and syntax
- `types.md` — complete type system
- `patterns.md` — full pattern matching
- `DASO.md` — memory management
- `moduling.md` — modules and imports