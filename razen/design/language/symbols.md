# Razen Language Specification: Symbols & Operators

**Core Rule:** `name: Type` everywhere — one separator, zero exceptions.

---

## 1. Variable & State Definitions

### 1.1 Immutable Declaration — `:=`

```razen
// Inferred
user_id := 1
ratio   := 0.75
lang    := "Razen"

// Explicit — name: Type := value
count:    int   := 42
name:     str   := "Razen"
pi:       float := 3.14159
pixel:    u8    := 255u8
weight:   f32   := 0.5f32
```

### 1.2 Mutable Declaration — `mut name: Type = value`

```razen
mut score:    int   = 0
mut username: str   = "guest"
mut cursor:   uint  = 0u
mut temp:     f32   = 36.6f32

score    = 99
username = "Alice"
```

### 1.3 Constants — `const name: Type = value`

```razen
const MAX_SCORE: int = 100
const APP_NAME:  str = "Razen Lang"
const PI:        f64 = 3.14159265358979
```

### 1.4 Compound Assignment Operators

Only valid on `mut` bindings.

| Symbol  | Meaning               | Example                |
|---------|-----------------------|------------------------|
| `+=`    | Add and assign        | `score += 10`          |
| `-=`    | Subtract and assign   | `score -= 3`           |
| `*=`    | Multiply and assign   | `score *= 2`           |
| `/=`    | Divide and assign     | `score /= 4`           |
| `%=`    | Modulo and assign     | `score %= 3`           |
| `**=`   | Exponent and assign   | `score **= 2`          |
| `+=`    | Append (strings)      | `tag += " world"`      |

```razen
mut n:   int   = 100
mut r:   float = 1.0
mut s:   str   = "hello"

n += 50     // 150
n -= 25     // 125
n *= 2      // 250
n /= 5      // 50
n %= 7      // 1
n **= 3     // 1

r += 0.5    // 1.5
r *= 2.0    // 3.0

s += " world"    // "hello world"
```

### 1.5 Async Pipeline — `~>`

Passes the result of the left expression to the next function.

```razen
async act process(url: str) result[str, str] {
    fetch(url) ~> parse() ~> validate() ~> save()
}
```

---

## 2. Structure, Scope & Access

### 2.1 Universal Dot Operator — `.`

```razen
user.name           // struct field
rect.origin.x       // nested field
std.io.println      // module function
Direction.North     // enum variant
text.parse_int()    // method call
arr.len()           // method call
```

### 2.2 Type Annotation — `:`

Universal separator between name and type. No exceptions.

```razen
count: int := 42                          // variable
mut score: int = 0                         // mutable variable

act greet(name: str, age: int) str { ... } // parameters

struct User {
    id:        int,                        // struct field
    name:      str,
    is_active: bool,
}

alice := User { id: 1, name: "Alice", is_active: true }  // instantiation

double := |x: int| x * 2                  // closure param
act apply(value: int, f: |int| -> int) int { f(value) }  // closure type
```

### 2.3 Scope Block — `{ }`

```razen
// Control flow
if score > 90 { println("A") }
loop i in 0..10 { println(i) }

// Match block
match status {
    Status.Active   -> println("active"),
    Status.Inactive -> println("inactive"),
}

// Block as expression — last expression is value
result := {
    x := compute_x()
    y := compute_y()
    x + y    // this is the block's value
}
```

### 2.4 Collection Brackets — `[ ]`

All collection types use `[]` for type parameters, generics, and literals.

```razen
// Fixed array
coords: [float; 3] := [0.0, 1.0, 0.0]

// vec
mut items: vec[int]  = vec[]
values    := vec[1, 2, 3, 4, 5]

// map — key: value inside []
mut scores: map[str, int] = map[]
ages        := map["Alice": 25, "Bob": 30]

// set
mut tags: set[str] = set[]
primes    := set[2, 3, 5, 7, 11]

// Generics
struct Box[T] { value: T }
act first[T](arr: vec[T]) option[T] { ... }

// Indexing and slicing
arr[0]
arr[1..4]     // exclusive slice
arr[1..=4]    // inclusive slice
arr[..3]      // prefix
arr[3..]      // suffix
arr[..]       // full view
```

### 2.5 Grouping & Arguments — `( )`

```razen
(a + b) * c              // math grouping

add(3, 4)                // function call

point  := (10, 20)       // tuple value
person := ("Alice", 25)  // tuple value

act div_mod(a: int, b: int) (int, int) {
    (a / b, a % b)
}

(quotient, remainder) := div_mod(17, 5)   // destructure
(x, y) := point
```

### 2.6 Struct Update — `..source`

Copy a struct with specific fields overridden. Remaining fields come from source.

```razen
alice := User { id: 1, name: "Alice", email: "alice@razen.dev", is_active: true }

// Override name only — other fields copied from alice
renamed     := User { name: "Alicia",        ..alice }
deactivated := User { is_active: false,      ..alice }
updated     := User { name: "Alicia", is_active: false, ..alice }
```

### 2.7 Label — `'name:` and `'name`

Labels target nested `break` and `next`.

```razen
// Label a loop
'outer: loop i in 0..10 {
    loop j in 0..10 {
        if i * j > 50 { break 'outer }    // exit 'outer
        if j % 2 == 0 { next 'outer }     // next 'outer iteration
    }
}

// break labeled loop with value
result := 'search: loop i in 0..100 {
    if check(i) { break 'search i }
} else {
    -1
}
```

---

## 3. Functions & Pattern Matching

### 3.1 Function Definition

```razen
act name(param: Type) ReturnType {
    body
}

// One-liner
act square(n: int) int -> n * n

// Generic
act identity[T](value: T) T -> value

// With where clause
act process[T, U](a: T, b: U) str
where
    T: Display,
    U: Debug,
{
    "{a.to_string()} / {b.debug_str()}"
}

// never — diverging function
act panic_with(msg: str) never {
    println("PANIC: {msg}")
    std.process.exit(1)
}
```

### 3.2 `->` — expression body and match arm

```razen
// One-liner function body
act double(n: int) int -> n * 2

// Match arm
area := match shape {
    Shape.Circle(r)       -> 3.14159 * r * r,
    Shape.Rectangle(w, h) -> w * h,
    _                      -> 0.0,
}
```

### 3.3 `ret` — early return

```razen
act clamp(x: int, lo: int, hi: int) int {
    if x < lo { ret lo }
    if x > hi { ret hi }
    x              // implicit return — last expression
}
```

### 3.4 Closures

```razen
// Typed params
double := |x: int| x * 2
add    := |a: int, b: int| a + b

// Inferred params (inline)
nums.map(|x| x * 2)
nums.filter(|x| x % 2 == 0)
nums.reduce(0, |acc, x| acc + x)

// Multi-line
clamp := |x: int| {
    if x < 0   { ret 0 }
    if x > 100 { ret 100 }
    x
}

// Closure type in parameter: |ParamType| -> ReturnType
act apply(value: int, f: |int| -> int) int { f(value) }

// Closure returning closure
act make_adder(n: int) |int| -> int { |x| x + n }
```

### 3.5 `_` — wildcard pattern

```razen
// catch-all in match (must be last)
match status {
    Status.Active -> "active",
    _             -> "other",
}

// ignore position in destructuring
(first, _, third) := (1, 2, 3)
{ name, _ } := user

// ignore a named field
match event {
    Event.Click { x, _ } -> handle_x(x),
    _                     -> {},
}

// discard unused value
_ := side_effect()
```

---

## 4. Arithmetic & Math

| Symbol  | Purpose              | Example          |
|---------|----------------------|------------------|
| `+`     | Add / concatenate    | `a + b`          |
| `-`     | Subtract / negate    | `a - b`, `-x`    |
| `*`     | Multiply             | `a * b`          |
| `/`     | Divide               | `a / b`          |
| `%`     | Modulo               | `a % b`          |
| `**`    | Exponentiation       | `2 ** 8` → 256   |

---

## 5. Comparison & Logic

| Symbol   | Purpose        | Example               |
|----------|----------------|-----------------------|
| `==`     | Equal          | `a == b`              |
| `!=`     | Not equal      | `a != b`              |
| `<`      | Less than      | `a < b`               |
| `>`      | Greater than   | `a > b`               |
| `<=`     | Less or equal  | `a <= b`              |
| `>=`     | Greater equal  | `a >= b`              |
| `&&`     | Logical AND    | `x > 0 && y > 0`     |
| `\|\|`   | Logical OR     | `x == 0 \|\| y == 0` |
| `!`    | Logical NOT    | `!is_valid`        |

```razen
if !is_valid { ret err("invalid") }
if x > 0 && !y.is_empty() { process(x, y) }
active := !user.is_inactive
```

---

## 6. Bitwise Operations

| Symbol | Purpose          |
|--------|------------------|
| `&`    | Bitwise AND      |
| `\|`   | Bitwise OR       |
| `^`    | XOR              |
| `~`    | Bitwise NOT      |
| `<<`   | Left shift       |
| `>>`   | Right shift      |

```razen
flags: u32 := 0b0000_1111u32
mask:  u32 := 0b1111_0000u32

combined := flags | mask     // 0b1111_1111
masked   := flags & mask     // 0b0000_0000
flipped  := ~flags           // 0b1111_0000 (inverted)
shifted  := flags << 4       // 0b1111_0000
```

---

## 7. Ranges & Iteration

| Symbol  | Meaning          | Example           |
|---------|------------------|-------------------|
| `..`    | Exclusive range  | `0..10` → 0–9    |
| `..=`   | Inclusive range  | `0..=10` → 0–10  |
| `in`    | Iteration / test | `loop i in 0..5`  |

```razen
loop i in 0..10   { println(i) }    // 0 to 9
loop i in 0..=10  { println(i) }    // 0 to 10

loop i in (0..50).step(10)  { println(i) }    // 0,10,20,30,40
loop i in (0..5).reverse()  { println(i) }    // 4,3,2,1,0

loop c in 'a'..='z' { print(c) }

slice := arr[1..4]      // exclusive
incl  := arr[1..=4]     // inclusive
pre   := arr[..3]       // prefix
suf   := arr[3..]       // suffix
all   := arr[..]        // whole

if role in allowed_roles { println("granted") }
if "rust" in tags        { println("found") }
```

---

## 8. Error Propagation — `?`

Propagates `err` or `none` as an early return from the current function.

```razen
act process(path: str) result[str, str] {
    data  := fs.read(path)?      // ret err(...) on failure
    value := parse(data)?
    ok(value)
}

act get_email(id: int) option[str] {
    user := find_user(id)?       // ret none if not found
    some(user.email)
}
```

---

## 9. Generics & Trait Bounds

| Syntax            | Meaning                               |
|-------------------|---------------------------------------|
| `[T]`             | Unconstrained type parameter          |
| `[T: Trait]`      | T must implement Trait                |
| `[T: A + B]`      | T must implement A and B              |
| `[T, U]`          | Two unconstrained parameters          |
| `[T: A, U: B]`    | Two constrained parameters            |
| `where T: A + B`  | Constraint in where clause            |

```razen
act identity[T](value: T) T -> value

act largest[T: Ord](a: T, b: T) T {
    if a.gt(b) { a } else { b }
}

act display_all[T: Display](items: vec[T]) void {
    loop item in items { println(item.to_string()) }
}

act complex[T, U](a: T, b: U) str
where
    T: Shape + Describable,
    U: Display,
{
    "{b.to_string()}: {a.describe()}"
}
```

---

## 10. Type Casting — `as`

```razen
// Widening — always safe
small: i32  := 100i32
big:   i64  := small as i64

byte:  u8   := 200u8
wide:  u32  := byte as u32

// Narrowing — truncates
val:   i64  := 300i64
small8: i8  := val as i8       // 44 (300 % 256)

// int ↔ float
count: int   := 42
ratio: float := count as float // 42.0
back:  int   := 3.9 as int     // 3 (truncates toward zero)

// char ↔ numeric
letter: char := 65u8 as char   // 'A'
code:   u32  := 'Z' as u32     // 90

// f32 ↔ f64
hp: f32 := 0.5f32
dp: f64 := hp as f64
```

---

## 11. Attributes — `@`

```razen
@test
act test_something() void { assert 1 + 1 == 2 }

@inline
act hot_path(x: int) int -> x * x

@derive[Debug, Clone, Eq]
struct Config { host: str, port: int }

@json[rename_all: "camelCase"]
struct Request { user_id: int, full_name: str }

@deprecated["use v2 API"]
act old_call() void { }

@extern["C"]
act c_func(n: int) int

@cfg[target: "linux"]
act platform_init() void { }
```

---

## 12. Comments

```razen
// Single-line comment

/*
 * Multi-line block comment
 */

/// Documentation comment — for public API docs
/// Supports markdown formatting.
pub act add(a: int, b: int) int -> a + b
```

---

## 13. String Interpolation — `{expr}`

```razen
name    := "Alice"
age     := 25

greeting := "Hello, {name}!"
calc     := "Next year: {age + 1}"
complex  := "Score: {score * 100.0 / max}%"
```

---

## 14. Memory & Unsafe

```razen
unsafe {
    ptr  := &value     // address-of
    data := *ptr       // dereference
    buf  := alloc(1024)
    free(buf)
}
```

---

## 15. Shared Ownership — `shared`

```razen
shared config = Config { host: "localhost", port: 8080 }
shared cache: map[str, str] = map[]

struct Node[T] {
    value: T,
    next:  option[shared Node[T]],
}

node_b := shared Node[int] { value: 2, next: none }
node_a := shared Node[int] { value: 1, next: some(node_b) }
```

---

## Complete Symbol Table

| Symbol     | Purpose                               | Example                              |
|------------|---------------------------------------|--------------------------------------|
| `:=`       | Immutable declare + init              | `count: int := 42`                   |
| `=`        | Assign (mutable only)                 | `score = 99`                         |
| `:`        | Name–type separator (universal)       | `name: str`, `id: int`               |
| `+=`       | Add and assign                        | `score += 10`                        |
| `-=`       | Subtract and assign                   | `score -= 3`                         |
| `*=`       | Multiply and assign                   | `score *= 2`                         |
| `/=`       | Divide and assign                     | `score /= 4`                         |
| `%=`       | Modulo and assign                     | `score %= 3`                         |
| `**=`      | Exponent and assign                   | `score **= 2`                        |
| `.`        | Universal access                      | `user.name`, `Status.Active`         |
| `->`       | Expression body / match arm           | `act sq(n: int) int -> n * n`        |
| `~>`       | Async pipeline                        | `fetch(url) ~> parse() ~> save()`   |
| `?`        | Error propagation                     | `data := fs.read(path)?`             |
| `!`      | Logical NOT                           | `!is_valid`                       |
| `..`       | Exclusive range / struct update rest  | `0..10`, `..alice`                   |
| `..=`      | Inclusive range                       | `0..=10`                             |
| `in`       | Membership / iteration                | `loop x in arr`, `x in set`         |
| `as`       | Type cast                             | `score as float`                     |
| `is`       | Type check                            | `value is int`                       |
| `_`        | Wildcard / discard                    | `(first, _, third)`                  |
| `'label:`  | Loop label                            | `'outer: loop i in 0..10 { }`       |
| `break 'l` | Break labeled loop                    | `break 'outer`                       |
| `next 'l`  | Next on labeled loop                  | `next 'outer`                        |
| `{expr}`   | String interpolation                  | `"Hello, {name}!"`                   |
| `ret`      | Early return                          | `ret err("invalid")`                 |
| `**`       | Exponentiation                        | `2 ** 8`                             |
| `[T]`      | Generic type parameter                | `vec[T]`                             |
| `[T: A]`   | Generic with bound                    | `[T: Shape]`                         |
| `[T: A+B]` | Multiple bounds                       | `[T: Shape + Describable]`           |
| `@name`    | Attribute (zero-arg)                  | `@test`, `@inline`                   |
| `@name[…]` | Attribute with args                   | `@derive[Debug, Clone]`              |
| `///`      | Documentation comment                 | `/// Computes the area.`             |

---

## Design Principles

1. **One separator** — `:` means "this is the type" everywhere.
2. **One bracket for collections** — `[]` for all type params and generics.
3. **No parentheses in `if`/`loop`** — conditions are clean.
4. **Trailing commas everywhere** — no friction when reordering.
5. **`!` over `not`** — familiar C-style short negation.
6. **Last expression is implicit return** — `ret` is for early exits only.
7. **Explicit over implicit** — `shared`, `mut`, `unsafe`, `pub` all require intent.

**See also:**
- `keywords.md` — language keywords
- `types.md` — complete type system
- `patterns.md` — pattern matching
- `DASO.md` — memory model