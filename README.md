# KHEmu: Binary Translation in Rust

KHEmu is an emulator that works like a compiler.  Started originally as the term project for the course "Compiler Lab (Honor Track)" at Peking University, KHEmu aims to help improve the performance of foreign-architecture code translation.  Quoting from the term project report,

> QEMU's userspace emulation comes with a limitation though; during the lifetime of a guest program, only the parts in the kernel are executed natively.  This is not ideal for computation-heavy activities such as graphics rendering or scientific computing.  In this work, we introduce the idea of dynamic linking to userspace emulation to foster faster emulation by executing more of the computation natively.  The prototype implementation for the idea, KHEmu, is written in the Rust programming language and is under active development.

The full report at submission state is hosted on [Google Docs](https://docs.google.com/document/d/1E8nGi2ca_9TGM_ECLdPZfSFOVbdYuNBzf2mjqYu95wI/edit?usp=sharing) and will not be updated any further.  The following milestone list is updated in real time.

- [x] IR generation via macro
- [x] ARMv8 aarch64 frontend (partial)
- [x] Speculative disassembly
- [x] DumpIR dummy backend for IR printout
- [x] LLVM backend (partial)
- [x] Static ELF loading and guest mapping
- [x] Guest code execution, guest register dump
- [x] LOOKUP_TB trap handling
- [ ] LLVM branch generation
- [ ] Guest stack setup, syscall proxy
- [ ] Block chaining
- [ ] Dynamic ELF loading
    - [ ] Parse and load dependent libraries
    - [ ] Host-side stub generation, DYNAMIC trap handling
    - [ ] Chaining
- [ ] Frontend support for FP & vector

## How to test

To run the project, you'll need Rust Nightly (as of June 2020).  The regular compiler can be simply retrieved with [Rustup](https://www.rust-lang.org/tools/install), while [enabling nightly](https://github.com/rust-lang/rustup/blob/master/README.md#working-with-nightly-rust) may require a bit more work.  Make sure you have LLVM 10 installed.  The code is only tested to work on Linux.  At present, you also need a statically-linked test executable for AArch64; a "Hello, world!" ELF is included in the submission archive (not committed to GitHub).  To run a simple test:

```bash
# clone the repo
git clone https://github.com/KireinaHoro/khemu && cd khemu
# setup LLVM for Rust
export LLVM_SYS_100_STRICT_VERSIONING=1
export LLVM_SYS_100_PREFIX=<your llvm 10 prefix>
# use a higher log level to see diagnosis information
export RUST_LOG=debug
# assume that `hello` is the test executable
cargo run hello
```

The test is expected to fail, likely panicking with the following message.  The failing point at submission is `host::llvm::make_label`, which is part of the LLVM branch generation milestone.

```text
thread 'main' panicked at 'not implemented', src/host/llvm.rs:313:9
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## Documentation

The project is documented via `rustdoc`.  To generate documentation locally,

```bash
# setup LLVM for Rust
export LLVM_SYS_100_STRICT_VERSIONING=1
export LLVM_SYS_100_PREFIX=<your llvm 10 prefix>
# assume that `hello` is the test executable
cargo doc
```

The documentation will be generated in `target/doc/khemu/index.html`.  A generated copy is included in the submitted archive (not committed to GitHub).

## License

This project is licensed under the 3-Clause BSD License.  See the `LICENSE` file for more information.
