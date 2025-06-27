# Contributing to CliqueFusion (C# Wrapper)

Thanks for contributing! This directory contains the C# wrapper around the native Rust-based `clique-fusion` library. It provides high-level, safe bindings to the native code and packages it for consumption via NuGet.

This guide describes how to build the project, run tests, and validate that native integration is working correctly.

---

## 🛠 Prerequisites

Before building or testing, ensure you have the following installed:

- [.NET 6 or later](https://dotnet.microsoft.com/)
- [Rust and Cargo](https://rustup.rs/)
- [`cross`](https://github.com/cross-rs/cross) for cross-compiling native libraries:

```bash
  cargo install cross
```

You should be able to run these successfully:

```bash
dotnet --version
cargo --version
cross --version
```

---

## 🔧 Native Build Process (Rust FFI)

The native FFI component is written in Rust and built automatically as part of the C# build process.

When you build `CliqueFusion.csproj`, it triggers `build-native.sh` as a pre-build step. This script:

- Uses `cross` to compile the Rust FFI target for:
  - `x86_64-unknown-linux-gnu` → `libclique_fusion_ffi.so`
  - `x86_64-pc-windows-gnu` → `clique_fusion_ffi.dll`
- Copies the resulting binaries into:

```
CliqueFusion/runtimes/{linux-x64,win-x64}/native/
```

These folders are used by the `.csproj` to embed native assets in the NuGet package.

---

## ✅ Running Unit Tests

You can run tests at any time — the native library will be built automatically as needed:

```bash
dotnet test
```

This runs tests covering:

- Native interop correctness (`CliqueIndexNative`)
- High-level wrapper behavior (`CliqueIndex`)
- Error handling, memory management, and clique logic

To collect code coverage:

```bash
dotnet test --collect:"XPlat Code Coverage"
```

Use `reportgenerator` to format the results if needed.

---

## ✅ Running the Smoke Test

The smoke test validates that the `.nupkg` works in a fresh consuming app — including native runtime probing.

### 1. Build the NuGet package:

```bash
dotnet pack src/CliqueFusion -c Release --output ./nupkgs
```

### 2. Run the smoke test:

```bash
dotnet run --project src/CliqueFusion.SmokeTest
```

If the package is broken (e.g. missing a native library), the test will fail with a runtime error.

---

## 🤝 Conventions

- `CliqueFusion.csproj` targets `net6.0` for broad compatibility
- Tests and smoke tests may use later frameworks like `net8.0`
- No need to build the Rust FFI manually — it’s built automatically
- Always run tests from the root (`dotnet test`) or directly per project
