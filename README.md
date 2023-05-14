# R2

A RISC-V emulator written in Rust :crab:.  
Inspired [cnlohr/mini-rv32ima](https://github.com/cnlohr/mini-rv32ima).

## Capture

You can run linux in your browser.

![capture](https://github.com/bokuweb/r2/blob/main/capture.gif?raw=true)

## Playground

[https://bokuweb.github.io/r2/](https://bokuweb.github.io/r2/)

## Native

```sh
$ cargo run -p app -- -i fixtures/linux.bin -d fixtures/default.dtb
```

## WASI

```sh
$ cargo build -p wasi --target wasm32-wasi --release
$ wasmtime ./target/wasm32-wasi/release/wasi.wasm
```

## Wasm

```sh
$ cd wasm
$ cargo build --target wasm32-unknown-unknown --release
$ wasm-opt --asyncify --pass-arg=asyncify-imports@env.keydown ../target/wasm32-unknown-unknown/release/wasm.wasm -o out.wasm
$ npx serve
```

## Special Thanks

- [cnlohr/mini-rv32ima](https://github.com/cnlohr/mini-rv32ima)
- [Writing a Really Tiny RISC-V Emulator](https://www.youtube.com/watch?v=YT5vB3UqU_E)

## References

- [https://github.com/torvalds/linux/tree/master/arch/riscv](https://github.com/torvalds/linux/tree/master/arch/riscv)
- [https://www.five-embeddev.com/riscv-isa-manual/latest/machine.html](https://www.five-embeddev.com/riscv-isa-manual/latest/machine.html)
- [https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)
- [https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf](https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf)

## License

MIT
