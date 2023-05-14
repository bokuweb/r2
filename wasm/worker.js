let last = performance.now();
const elapsed_us = () => {
  const elapsed = (performance.now() - last) * 1000;
  last = performance.now();
  return elapsed;
};

const tx = (s) => {
  postMessage(s);
};

const keybuf = [];
const keydown = async () => {
  if (!!keybuf.length) return true;
  await delay();
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

async function load(module, imports) {
  if (typeof Response === "function" && module instanceof Response) {
    try {
      return await instantiateStreaming(module, imports);
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
  imports.env = {};
  imports.env.elapsed_us = elapsed_us;
  imports.env.tx = function (arg0) {
    tx(arg0 >>> 0);
  };
  imports.env.keydown = keydown;
  imports.env.rx = rx;

  return imports;
}

function finalizeInit(instance, module) {
  wasm = instance.exports;
  init.__wbindgen_wasm_module = module;
  cachedUint8Memory0 = null;

  return wasm;
}

async function init(input) {
  if (typeof input === "undefined") {
    input = "out.wasm";
  }
  const imports = getImports();

  if (
    typeof input === "string" ||
    (typeof Request === "function" && input instanceof Request) ||
    (typeof URL === "function" && input instanceof URL)
  ) {
    input = fetch(input);
  }
  const { instance, module } = await load(await input, imports);

  return finalizeInit(instance, module);
}

const delay = () =>
  new Promise((r) =>
    setTimeout(() => {
      r();
    })
  );

init().then((w) => {
  w.start();
});

// Asyncify
const t = new WeakMap();
function e(t, e) {
  return new Proxy(t, { get: (t, r) => e(t[r]) });
}
class r {
  constructor() {
    (this.value = void 0), (this.exports = null);
  }
  getState() {
    return this.exports.asyncify_get_state();
  }
  assertNoneState() {
    let t = this.getState();
    if (0 !== t) throw new Error(`Invalid async state ${t}, expected 0.`);
  }
  wrapImportFn(t) {
    return (...e) => {
      if (2 === this.getState())
        return this.exports.asyncify_stop_rewind(), this.value;
      this.assertNoneState();
      let r = t(...e);
      if (
        !(s = r) ||
        ("object" != typeof s && "function" != typeof s) ||
        "function" != typeof s.then
      )
        return r;
      var s;
      this.exports.asyncify_start_unwind(16), (this.value = r);
    };
  }
  wrapModuleImports(t) {
    return e(t, (t) => ("function" == typeof t ? this.wrapImportFn(t) : t));
  }
  wrapImports(t) {
    if (void 0 !== t)
      return e(t, (t = Object.create(null)) => this.wrapModuleImports(t));
  }
  wrapExportFn(e) {
    let r = t.get(e);
    console.log({ r, e: e.toString() });
    return (
      void 0 !== r ||
        ((r = async (...t) => {
          this.assertNoneState();
          let r = e(...t);
          for (; 1 === this.getState(); )
            this.exports.asyncify_stop_unwind(),
              (this.value = await this.value),
              this.assertNoneState(),
              this.exports.asyncify_start_rewind(16),
              (r = e());
          return this.assertNoneState(), r;
        }),
        t.set(e, r)),
      r
    );
  }
  wrapExports(e) {
    console.log(e);
    let r = Object.create(null);
    for (let t in e) {
      let s = e[t];
      "function" != typeof s ||
        t.startsWith("asyncify_") ||
        (s = this.wrapExportFn(s)),
        Object.defineProperty(r, t, { enumerable: !0, value: s });
    }
    return t.set(e, r), r;
  }
  init(t, e) {
    const { exports: r } = t,
      n = r.memory || (e.env && e.env.memory);
    new Int32Array(n.buffer, 16).set([24, 1024]),
      (this.exports = this.wrapExports(r)),
      Object.setPrototypeOf(t, s.prototype);
  }
}
class s extends WebAssembly.Instance {
  constructor(t, e) {
    let s = new r();
    super(t, s.wrapImports(e)), s.init(this, e);
  }
  get exports() {
    return t.get(super.exports);
  }
}
async function n(t, e) {
  let s = new r(),
    n = await WebAssembly.instantiate(t, s.wrapImports(e));
  return s.init(n instanceof WebAssembly.Instance ? n : n.instance, e), n;
}
async function instantiateStreaming(t, e) {
  let s = new r(),
    n = await WebAssembly.instantiateStreaming(t, s.wrapImports(e));
  console.log(n);
  return s.init(n.instance, e), n;
}
