# ALL RYX DATA TYPES - Complete Examples (Spine Case)

---

## 1. PRIMITIVES

```razen
// bool
is_Active := true
has_Error := false

// int (64-bit signed)
count    := 42
negative := -100
hex      := 0xFF
binary   := 0b1010
octal    := 0o755

// uint (64-bit unsigned)
index    := 0u
size     := 1024u

// float (64-bit)
pi       := 3.14159
price    := 99.99
scientific := 1.5e-10

// str (UTF-8)
name     := "Razen"
emoji    := "🚀💻"
template := "Hello, {name}!"

// bytes (raw binary)
data     := bytes[0x48, 0x65, 0x6C, 0x6C, 0x6F]
buffer   := bytes[1024]  // 1024 zero bytes
```

---

## 2. ARRAYS - Fixed Size

```razen
// Fixed-size array [type; size]
numbers  := [int; 5]           // [0, 0, 0, 0, 0]
values   := [1, 2, 3, 4, 5]    // Type inferred: [int; 5]

// Multi-dimensional
matrix   := [[int; 3]; 3]      // 3x3 matrix
grid     := [[1, 2, 3],
             [4, 5, 6],
             [7, 8, 9]]

// Array operations
arr := [10, 20, 30, 40, 50]

first     := arr[0]              // 10
last      := arr[arr.len() - 1]  // 50
slice     := arr[1..4]           // [20, 30, 40]
slice_Inc := arr[1..=4]          // [20, 30, 40, 50]

// Iteration
loop item in arr {
    println(item)
}

loop (index, value) in arr.enumerate() {
    println("arr[{index}] = {value}")
}
```

---

## 3. VECTORS - Dynamic Size

```razen
// vec[T] - growable array
mut nums := vec[int]()

// Add elements
nums.push(10)
nums.push(20)
nums.push(30)

// Initialize with values
values := vec[1, 2, 3, 4, 5]

// vec operations
nums.pop()              // Remove last → some(30)
nums.insert(1, 15)      // Insert at index
nums.remove(0)          // Remove at index
nums.clear()            // Remove all

length   := nums.len()
is_Empty := nums.is_Empty()

// Access
first := nums[0]
last  := nums[nums.len() - 1]

// With capacity
mut buffer := vec[int].with_Capacity(100)

// Vector of structs
mut users := vec[user]()
users.push(user { id 1, name "Alice" })
users.push(user { id 2, name "Bob" })

// Nested vectors
mut matrix := vec[vec[int]]()
matrix.push(vec[1, 2, 3])
matrix.push(vec[4, 5, 6])
```

---

## 4. MAPS - Key-Value Storage

```razen
// map[K, V]
mut scores := map[str, int]()

// Insert
scores.insert("Alice", 95)
scores.insert("Bob", 87)
scores.insert("Charlie", 92)

// Initialize with values
ages := map {
    "Alice"   25,
    "Bob"     30,
    "Charlie" 28,
}

// Access
alice_Score := scores.get("Alice")  // option[int] → some(95)
unknown     := scores.get("Dave")   // none

// Get with default
score := scores.get("Alice").unwrap_or(0)

// Update
scores.insert("Alice", 98)  // Replaces 95

// Remove
scores.remove("Bob")  // Removes Bob

// Check existence
has_Alice := scores.contains("Alice")

// Iteration
loop (key, value) in scores {
    println("{key}: {value}")
}

// Just keys
loop key in scores.keys() {
    println(key)
}

// Just values
loop value in scores.values() {
    println(value)
}

// Map operations
size := scores.len()
scores.clear()

// Complex maps
mut cache := map[str, user]()
cache.insert("user_1", user { id 1, name "Alice" })

mut nested := map[str, vec[int]]()
nested.insert("primes", vec[2, 3, 5, 7, 11])
```

---

## 5. SETS - Unique Values

```razen
// set[T]
mut tags := set[str]()

// Add
tags.insert("rust")
tags.insert("python")
tags.insert("go")
tags.insert("rust")  // Duplicate ignored

// Initialize
languages := set["Razen", "Rust", "Go", "Python"]

// Check membership
has_Rust := tags.contains("rust")  // true
has_Java := tags.contains("java")  // false

// Remove
tags.remove("python")

// Set operations
set1 := set[1, 2, 3, 4]
set2 := set[3, 4, 5, 6]

union        := set1.union(set2)        // {1,2,3,4,5,6}
intersection := set1.intersection(set2) // {3,4}
difference   := set1.difference(set2)   // {1,2}

// Iteration
loop item in tags {
    println(item)
}

// Set of structs (must implement Hash + Eq)
mut user_Set := set[user_Id]()
user_Set.insert(123)
user_Set.insert(456)
```

---

## 6. TUPLES - Fixed Groups

```razen
// Tuple (unnamed fields)
point    := (10, 20)
person   := ("Alice", 25, true)
coords   := (1.5, 2.3, 4.7)

// Access by index
x := point.0    // 10
y := point.1    // 20

name   := person.0    // "Alice"
age    := person.1    // 25
active := person.2    // true

// Destructuring
(x, y) := point
(name, age, is_Active) := person

// Return multiple values
act divmod(a int, b int) (int, int) {
    retn (a / b, a % b)
}

(quotient, remainder) := divmod(17, 5)

// Tuple in struct
struct response {
    data   str
    status (int, str)  // (code, message)
}

response := response {
    data   "Success",
    status (200, "OK"),
}

// Nested tuples
nested := ((1, 2), (3, 4))
first  := nested.0     // (1, 2)
value  := nested.0.1   // 2
```

---

## 7. OPTION - Nullable Values

```razen
// option[T] - some(value) or none
act find_User(id int) option[user] {
    if id == 1 {
        retn some(user { id 1, name "Alice" })
    }
    retn none
}

// Using Option
result := find_User(1)

match result {
    some(user) -> println("Found: {user.name}"),
    none       -> println("Not found"),
}

// Unwrap methods
user := result.unwrap()           // Panics if none
user := result.unwrap_or(default) // Returns default if none
user := result.expect("No user")  // Panics with message

// Check if has value
if result.is_Some() {
    user := result.unwrap()
}

if result.is_None() {
    println("Empty")
}

// Map and transform
names := option[str]
    .some("alice")
    .map(|s| s.to_Upper_Case())  // some("ALICE")

// Option in collections
mut maybe_Values := vec[option[int]]()
maybe_Values.push(some(10))
maybe_Values.push(none)
maybe_Values.push(some(20))

// Filter out none
loop item in maybe_Values {
    if let some(value) = item {
        println(value)
    }
}
```

---

## 8. RESULT - Error Handling

```razen
// result[T, E] - ok(value) or err(error)
act parse_Age(input str) result[int, str] {
    age := input.parse_int()?
    
    if age < 0 {
        retn err("Negative age")
    }
    
    if age > 150 {
        retn err("Invalid age")
    }
    
    retn ok(age)
}

// Using Result
result := parse_Age("25")

match result {
    ok(age)  -> println("Age: {age}"),
    err(msg) -> eprintln("Error: {msg}"),
}

// Unwrap methods
age := result.unwrap()           // Panics if err
age := result.unwrap_or(0)       // Default if err
age := result.expect("Invalid")  // Panics with message

// Check result
if result.is_Ok() {
    age := result.unwrap()
}

if result.is_Err() {
    error := result.unwrap_Err()
}

// Map and transform
result := ok(10)
    .map(|x| x * 2)           // ok(20)
    .map_Err(|e| "Error: {e}") // Transform error

// Chaining with ?
act process() result[int, str] {
    age  := parse_Age(input())?
    user := create_User(age)?
    id   := save(user)?
    retn ok(id)
}
```

---

## 9. STRINGS - Text Operations

```razen
// String creation
text := "Hello, Razen!"
empty := ""
multiline := "Line 1
Line 2
Line 3"

// Interpolation
name := "Alice"
age  := 25
msg  := "Name: {name}, Age: {age}"

// String operations
length   := text.len()              // 11
is_Empty := text.is_Empty()         // false

upper := text.to_Upper_Case()       // "HELLO, RYX!"
lower := text.to_Lower_Case()       // "hello, razen!"

trimmed := "  spaces  ".trim()    // "spaces"

// Contains and search
has_Razen  := text.contains("Razen")    // true
starts_H := text.starts_With("Hello") // true
ends_Ex  := text.ends_With("!")      // true

index := text.find("Razen")         // some(7)

// Split and join
parts  := "a,b,c,d".split(",")     // ["a", "b", "c", "d"]
joined := parts.join("-")         // "a-b-c-d"

lines := multiline.split("\n")

// Replace
replaced := text.replace("Razen", "World")

// Substring
slice := text[0..5]               // "Hello"
chars := text.chars()             // Iterator of chars

// String builder
mut builder := string_Builder()
builder.append("Hello")
builder.append(" ")
builder.append("World")
result := builder.to_String()      // "Hello World"

// Raw strings (no escaping)
path  := r"C:\Users\Razen\file.txt"
regex := r"\d+\.\d+"
```

---

## 10. SLICES - Views into Arrays/Vecs

```razen
// Slice - borrowed view
arr := [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// Slice syntax
first5 := arr[0..5]      // [1, 2, 3, 4, 5]
middle := arr[3..7]      // [4, 5, 6, 7]
last5  := arr[5..10]     // [6, 7, 8, 9, 10]

// Inclusive slicing
range := arr[2..=5]     // [3, 4, 5, 6]

// From start
prefix := arr[..4]       // [1, 2, 3, 4]

// To end
suffix := arr[5..]       // [6, 7, 8, 9, 10]

// Full slice
all := arr[..]        // [1, 2, 3, ..., 10]

// Slice a vec
mut vec := vec[1, 2, 3, 4, 5]
portion := vec[1..4]       // [2, 3, 4]

// Slice operations
length := portion.len()
first  := portion[0]

loop item in portion {
    println(item)
}
```

---

## 11. RANGES - Iterators

```razen
// Exclusive range
range1 := 0..10           // 0 to 9

loop i in range1 {
    println(i)
}

// Inclusive range
range2 := 0..=10          // 0 to 10

loop i in range2 {
    println(i)
}

// Step ranges (via iterator)
loop i in (0..100).step(10) {
    println(i)  // 0, 10, 20, ..., 90
}

// Reverse
loop i in (0..10).reverse() {
    println(i)  // 9, 8, 7, ..., 0
}

// Range in array indexing
arr := [10, 20, 30, 40, 50]
slice := arr[1..4]

// Char ranges
loop c in 'a'..'z' {
    print(c)  // a to y
}

loop c in 'a'..='z' {
    print(c)  // a to z
}
```

---

## 12. TENSORS - Multi-Dimensional Arrays (AI/ML)

```razen
// 1D tensor (vector)
vec := tensor[1.0, 2.0, 3.0, 4.0]

// 2D tensor (matrix)
matrix := tensor[
    [1.0, 2.0, 3.0],
    [4.0, 5.0, 6.0],
]

// 3D tensor
cube := tensor[
    [[1.0, 2.0], [3.0, 4.0]],
    [[5.0, 6.0], [7.0, 8.0]],
]

// Tensor operations
shape := matrix.shape()       // (2, 3)
size  := matrix.size()        // 6
dtype := matrix.dtype()       // float

// Reshaping
reshaped := matrix.reshape([3, 2])

// Math operations
a := tensor[1.0, 2.0, 3.0]
b := tensor[4.0, 5.0, 6.0]

sum  := a + b                 // Element-wise
prod := a * b                 // Element-wise
dot  := a.dot(b)              // Dot product

// Matrix multiplication
m1 := tensor[[1.0, 2.0], [3.0, 4.0]]
m2 := tensor[[5.0, 6.0], [7.0, 8.0]]
result := m1.matmul(m2)

// Indexing
value := matrix[0, 1]         // 2.0
row   := matrix[0]            // tensor[1.0, 2.0, 3.0]

// Slicing
submatrix := matrix[0..1, 1..3]

// GPU acceleration
gpu_Tensor := tensor[1.0, 2.0, 3.0].to_Gpu()
result := gpu_Tensor * 2.0
cpu_Result := result.to_Cpu()
```

---

## 13. CUSTOM COLLECTIONS

```razen
// Stack
struct stack[T] {
    items vec[T]
}

impl stack[T] {
    act new() stack[T] {
        retn stack { items vec[T]() }
    }
    
    act push(mut self, item T) {
        self.items.push(item)
    }
    
    act pop(mut self) option[T] {
        retn self.items.pop()
    }
    
    act peek(self) option[T] {
        retn self.items.last()
    }
}

// Queue
struct queue[T] {
    items vec[T]
}

impl queue[T] {
    act new() queue[T] {
        retn queue { items vec[T]() }
    }
    
    act enqueue(mut self, item T) {
        self.items.push(item)
    }
    
    act dequeue(mut self) option[T] {
        if self.items.is_Empty() {
            retn none
        }
        retn some(self.items.remove(0))
    }
}

// LinkedList
struct node[T] {
    value T
    next  option[shared node[T]]
}

struct linked_List[T] {
    head option[shared node[T]]
    size int
}
```

---

## 14. COMPLETE DATA STRUCTURE EXAMPLE

```razen
struct database {
    users    map[int, user]
    sessions map[str, session]
    tags     set[str]
    logs     vec[log_Entry]
    cache    map[str, bytes]
}

struct user {
    id       int
    name     str
    email    str
    roles    set[role]
    metadata map[str, str]
}

struct session {
    token      str
    user_Id    int
    created_At int
    expires_At int
    data       map[str, str]
}

struct log_Entry {
    timestamp int
    level     log_Level
    message   str
    context   map[str, str]
}

enum log_Level {
    debug,
    info,
    warning,
    error,
}

enum role {
    admin,
    moderator,
    user,
}

act main() {
    mut db := database {
        users    map[int, user](),
        sessions map[str, session](),
        tags     set[str](),
        logs     vec[log_Entry](),
        cache    map[str, bytes](),
    }
    
    // Add user
    user := user {
        id       1,
        name     "Alice",
        email    "alice@razen.dev",
        roles    set[role.admin, role.user],
        metadata map {
            "country" "USA",
            "city"    "SF",
        },
    }
    
    db.users.insert(user.id, user)
    
    // Add session
    session := session {
        token      "abc123",
        user_Id    1,
        created_At now(),
        expires_At now() + 3600,
        data       map[str, str](),
    }
    
    db.sessions.insert(session.token, session)
    
    // Add tags
    db.tags.insert("active")
    db.tags.insert("premium")
    
    // Add log
    entry := log_Entry {
        timestamp now(),
        level     log_Level.info,
        message   "User logged in",
        context   map {
            "user_Id" "1",
            "ip"      "192.168.1.1",
        },
    }
    
    db.logs.push(entry)
}
```

---

**That's ALL data types with Spine Case naming!** 