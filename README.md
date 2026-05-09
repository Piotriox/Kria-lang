# Kria Programming Language

A custom programming language written in Rust, featuring a flat bytecode VM with combined loop instructions for high-performance execution.

## Features

- File extension: `.krx`
- Flat bytecode VM with constant pool and combined instructions
- Dynamic + strong typing
- Variable assignment with `set`
- Arithmetic operations (+, -, *, /)
- Comparison operators: `==`, `!=`, `>`, `<`, `>=`, `<=`
- Logical operators: `and`, `or`, `not`
- Conditional branches: `if`, `elseif`, `else`
- Loops: `while`
- Block scope with `{}`
- Line comments using `//`
- Print function
- Null handling for undefined variables
- Newline-based statement termination (no semicolons)

## Architecture

Kria compiles source into flat bytecode (u8 opcodes + constant pool) before execution on a stack-based VM. The compiler emits combined instructions for hot paths like `while (var < N) { var = var + 1 }`, reducing dispatch from 6 to 1 per iteration.

```text
Source → Lexer → Parser → Compiler → Flat Bytecode VM
```

### Performance

| Test | Avg Execution Time |
|------|-------------------|
| `perf_test.krx` (1M loop) | ~12ms |
| `test.krx` (general features) | ~5ms |

*Measured with `cargo build --release` on warm cache.*

## Building

Ensure you have Rust installed. Then:

```bash
# Debug build (for development)
cargo build

# Release build (for performance — recommended)
cargo build --release
```

## Running

```bash
# Development
cargo run -- test.krx

# Release (recommended for benchmarking)
cargo run --release -- test.krx

# Or run the binary directly after release build
./target/release/kria test.krx
```

Example `test.krx`:
```kria
set x = 5
set x = 4
set y = 3
set text = "test"
print(x + y)
print(text)

set mynum = true
if (mynum == true) {
   print("Mynum is true")
} elseif (mynum == false) {
   print("Mynum is false")
} else {
   print("mynum must be a boolean")
}

set counter = 0
while (counter < 5) {
    print(counter)
    set counter = counter + 1
}

// Line comments start with // and continue to the end of the line.
```

## Installation

### Windows (creates kria-setup.exe)
```powershell
# Requires NSIS: https://nsis.sourceforge.io/
powershell scripts/build-windows.ps1
# Run kria-setup.exe to install
```

### Linux/macOS (creates kria-setup.sh)
```bash
./scripts/build-linux.sh
sudo ./dist/kria-setup.sh /usr/local
```

After installation, use:
```bash
kria test.krx
```

## License
MIT