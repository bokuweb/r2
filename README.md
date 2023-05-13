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
$ cd wasm && npx serve
```

### Build

```sh
$ cd wasm && wasm-pack build --target web
$ wasm-opt --asyncify --pass-arg=asyncify-imports@wbg.__wbg_keydown_a74a85d9b977730c pkg/wasm_bg.wasm -o out.wasm
```

## Special Thanks

- [cnlohr/mini-rv32ima](https://github.com/cnlohr/mini-rv32ima)
- [Writing a Really Tiny RISC-V Emulator](https://www.youtube.com/watch?v=YT5vB3UqU_E)

## References

- [https://github.com/torvalds/linux/tree/master/arch/riscv](https://github.com/torvalds/linux/tree/master/arch/riscv)
- [https://www.five-embeddev.com/riscv-isa-manual/latest/machine.html](https://www.five-embeddev.com/riscv-isa-manual/latest/machine.html)
- [https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf](https://github.com/riscv/riscv-isa-manual/releases/download/Priv-v1.12/riscv-privileged-20211203.pdf)

## License

MIT
