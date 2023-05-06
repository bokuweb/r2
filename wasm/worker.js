let last = performance.now();
const elapsed_us = () => {
  const elapsed = (performance.now() - last) * 1000;
  last = performance.now();
  return elapsed;
};

const wait = (us) => {
  const start = performance.now();
  let current = start;
  const target = start + us / 1000;
  while (current < target) {
    current = performance.now();
  }
};

let displayed = "";
const tx = (s) => {
  displayed += String.fromCharCode(s);
  if (displayed.includes("login:")) {
    ready = true;
  }
  postMessage(s);
};

let ready = false;
const keybuf = [];
const keydown = () => {
  return !!keybuf.length;
};
const rx = () => {
  const d = keybuf.shift();
  return d.charCodeAt(0);
};
self.addEventListener("message", (event) => {
  keybuf.push(event.data);
});

let wasm;

const cachedTextDecoder = new TextDecoder("utf-8", {
  ignoreBOM: true,
  fatal: true,
});

cachedTextDecoder.decode();

let cachedUint8Memory0 = null;

function getUint8Memory0() {
  if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
    cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
  }
  return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
  return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

function notDefined(what) {
  return () => {
    throw new Error(`${what} is not defined`);
  };
}
/**
 */
class WasmCore {
  static __wrap(ptr) {
    const obj = Object.create(WasmCore.prototype);
    obj.ptr = ptr;

    return obj;
  }

  __destroy_into_raw() {
    const ptr = this.ptr;
    this.ptr = 0;

    return ptr;
  }

  free() {
    const ptr = this.__destroy_into_raw();
    wasm.__wbg_wasmcore_free(ptr);
  }
  /**
   * @returns {WasmCore}
   */
  static new() {
    const ret = wasm.wasmcore_new();
    return WasmCore.__wrap(ret);
  }
  /**
   */
  step() {
    wasm.wasmcore_step(this.ptr);
  }
}

async function load(module, imports) {
  if (typeof Response === "function" && module instanceof Response) {
    if (typeof WebAssembly.instantiateStreaming === "function") {
      try {
        return await WebAssembly.instantiateStreaming(module, imports);
      } catch (e) {
        if (module.headers.get("Content-Type") != "application/wasm") {
          console.warn(
            "`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n",
            e
          );
        } else {
          throw e;
        }
      }
    }

    const bytes = await module.arrayBuffer();
    return await WebAssembly.instantiate(bytes, imports);
  } else {
    const instance = await WebAssembly.instantiate(module, imports);

    if (instance instanceof WebAssembly.Instance) {
      return { instance, module };
    } else {
      return instance;
    }
  }
}

function getImports() {
  const imports = {};
  imports.wbg = {};
  imports.wbg.__wbg_elapsedus_06ff79887b966e05 =
    typeof elapsed_us == "function" ? elapsed_us : notDefined("elapsed_us");
  imports.wbg.__wbg_tx_97f8eca74be106c3 = function (arg0) {
    tx(arg0 >>> 0);
  };
  imports.wbg.__wbg_keydown_a74a85d9b977730c =
    typeof keydown == "function" ? keydown : notDefined("keydown");
  imports.wbg.__wbg_rx_872d8f80b416285d =
    typeof rx == "function" ? rx : notDefined("rx");
  imports.wbg.__wbg_wait_e8341c9dc02f44b6 = function (arg0) {
    wait(arg0 >>> 0);
  };
  imports.wbg.__wbindgen_throw = function (arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
  };

  return imports;
}

function initMemory(imports, maybe_memory) {}

function finalizeInit(instance, module) {
  wasm = instance.exports;
  init.__wbindgen_wasm_module = module;
  cachedUint8Memory0 = null;

  return wasm;
}

function initSync(module) {
  const imports = getImports();

  initMemory(imports);

  if (!(module instanceof WebAssembly.Module)) {
    module = new WebAssembly.Module(module);
  }

  const instance = new WebAssembly.Instance(module, imports);

  return finalizeInit(instance, module);
}

async function init(input) {
  if (typeof input === "undefined") {
    input = "wasm_bg.wasm";
  }
  const imports = getImports();

  if (
    typeof input === "string" ||
    (typeof Request === "function" && input instanceof Request) ||
    (typeof URL === "function" && input instanceof URL)
  ) {
    input = fetch(input);
  }

  initMemory(imports);

  const { instance, module } = await load(await input, imports);

  return finalizeInit(instance, module);
}

const delay = () => new Promise((r) => setTimeout(r));

init().then(async () => {
  const core = WasmCore.new();
  while (true) {
    if (ready) {
      for (let i = 0; i < 30000; i++) {
        core.step();
      }
      // poll post messages
      await delay();
    } else {
      core.step();
    }
  }
});
