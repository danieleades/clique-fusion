# CliqueFusion

**CliqueFusion** is a high-level C# wrapper around a native Rust library that performs spatial clustering of observations into maximal cliques using exact arithmetic and chi-squared compatibility testing.

It allows C# applications to efficiently:

- Insert 2D observations with uncertainty
- Group them into statistically compatible cliques
- Perform incremental updates or batch processing

The native core is implemented in Rust and exposed to C# via a C-compatible FFI layer.

---

## 🚀 Example: Usage in C#

```csharp
using CliqueFusion;

// Define a few observations
var obs1 = new Observation(Guid.NewGuid(), 1.0, 2.0, 1.0, 0.0, 1.0);
var obs2 = new Observation(Guid.NewGuid(), 1.1, 2.1, 1.0, 0.0, 1.0);
var obs3 = new Observation(Guid.NewGuid(), 5.0, 5.0, 1.0, 0.0, 1.0);

// Use a predefined chi-squared threshold (95% confidence)
double threshold = CliqueThresholds.Confidence95;

// Create the clique index
using var index = new CliqueIndex(new[] { obs1, obs2, obs3 }, threshold);

// Query cliques
var cliques = index.GetCliques();

foreach (var clique in cliques)
{
    Console.WriteLine($"Clique with {clique.ObservationIds.Count} members:");
    foreach (var id in clique.ObservationIds)
        Console.WriteLine($"  - {id}");
}
```

---

## 🔍 Functionality

- Supports insertion of 2D observations with full covariance matrices
- Handles optional `context` UUIDs to prevent self-merging
- Returns maximal cliques of mutually compatible observations
- Provides high-level C# records and APIs with minimal overhead
- Backed by robust native code implemented in Rust

---

## 🛠 Build Instructions

The C# library depends on a native Rust component (`clique-fusion-ffi`) compiled as a `.so` or `.dll`. You must build this before building or running tests.

### 1. Prerequisites

- [.NET 6+ SDK](https://dotnet.microsoft.com/)
- [Rust + cargo](https://rustup.rs/)

### 2. Build the native libraries

From the `CliqueFusion` directory:

```bash
./build-native.sh
```

This compiles the native library for your current platform and copies it into `CliqueFusion/runtimes/<rid>/native/`.

### 3. Build the C# library

```bash
dotnet build
```

---

## ✅ Running Tests

The native library is built automatically as part of the build, so you can run:

```bash
dotnet test
```

Test coverage is tracked using `coverlet.collector`. To see a summary:

```bash
dotnet test --collect:"XPlat Code Coverage"
```

---

## 📦 Folder Structure

```
CliqueFusion/
├── build-native.sh        # Builds Rust FFI for current platform
├── CliqueFusion.csproj    # Main C# wrapper
├── Native/                # P/Invoke layer
├── Wrappers/              # High-level C# APIs
└── runtimes/              # Platform-specific native binaries
```

---

## 🧪 See Also

- Rust core: [clique-fusion](../../../)
- C ABI layer: [clique-fusion-ffi](../../../clique-fusion-ffi)
