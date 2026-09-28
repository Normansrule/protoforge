# forge-server — the Protoforge engine over HTTP

A local representational state transfer (REST) server around `forge-core`.
It speaks the **same JSON method API** the browser reaches through
WebAssembly (`call(method, input)`), so Python notebooks, shell scripts,
continuous integration (CI) jobs or other tools can drive every lab headlessly.
Built on [axum](https://github.com/tokio-rs/axum) 0.8 and Tokio; engine calls
run on Tokio's blocking thread pool.

```bash
# from the repository root
cargo run --release -p forge-server -- --port 8787
# serve the built web app from the same origin as the API
./scripts/build-wasm.sh && (cd web && npm ci && npm run build)
cargo run --release -p forge-server -- --port 8787 --static web/dist
```

Options: `--host 127.0.0.1` (use `0.0.0.0` to expose on your network),
`--port 8787`, `--static DIR`, `--body-limit-mb 32`, `--ngspice-timeout 30`,
`--ngspice-bin ngspice`, `--no-cors`. Cross-origin resource sharing (CORS) is
open by default so a Vite dev server on another port can call it.

## Routes

| Method & path | Body | Response |
|---|---|---|
| `GET /api/health` | — | `{"status":"ok","version":…,"modules":[…],"ngspice":bool}` |
| `GET /api/methods` | — | `{"ok":["project","systems","circuit",…]}` |
| `POST /api/{method}` | JSON input (empty = `null`) | `200 {"ok":…}` · `400` invalid JSON · `404` unknown method/module · `422` engine rejected the input |
| `GET /api/{method}` | — | same as POST with a `null` input |
| `POST /api/package` | project JSON (empty = Iris) | `application/zip` fabrication package |
| `GET /api/package` | — | the Iris fabrication package |
| `POST /api/ngspice` | `{"netlist":"…"}` or raw netlist text | `{"ok":{"exit_code","success","stdout","stderr","elapsed_ms","raw_bytes","raw_base64"}}` · `501` when ngspice is not installed · `504` on timeout |
| `GET /*` | — | static files from `--static`, falling back to `index.html` |

Error bodies are always `{"error": "<message>"}`. Requests larger than the
body limit get `413 Payload Too Large`.

## curl examples

```bash
curl -s localhost:8787/api/health | jq

# fetch the Iris project, then verify every requirement
curl -s localhost:8787/api/project.iris | jq .ok > iris.json
curl -s -X POST localhost:8787/api/systems.evaluate \
     -H 'content-type: application/json' --data @iris.json \
  | jq '.ok | {pass, fail, unverified}'

# one engine call: IPC-2221 trace width for 1 A at a 10 °C rise
curl -s -X POST localhost:8787/api/pcb.ipc2221 -H 'content-type: application/json' \
     -d '{"current_a": 1.0, "temp_rise_c": 10, "copper_oz": 1, "external": true}' | jq

# the Markdown verification report
curl -s -X POST localhost:8787/api/systems.report -H 'content-type: application/json' \
     --data @iris.json | jq -r .ok

# download the complete fabrication package (Gerbers, drill, BOM, CPL, STL, G-code, guides …)
curl -s -o iris-fab-package.zip localhost:8787/api/package
curl -s -o my-fab.zip -X POST localhost:8787/api/package --data @my-product.json

# cross-check a Circuit Lab deck with a real SPICE (needs `sudo apt install ngspice`)
curl -s -X POST localhost:8787/api/ngspice --data-binary @divider.cir | jq -r .ok.stdout
```

Method names and input shapes are documented per lab in `docs/labs/*.md`;
the same calls work from the command-line interface (CLI):
`protoforge call pcb.ipc2221 '{"current_a":1}'`.

From Python:

```python
import requests
iris = requests.get("http://localhost:8787/api/project.iris").json()["ok"]
iris["design"]["frame"]["bridge_mm"] = 20
ev = requests.post("http://localhost:8787/api/systems.evaluate", json=iris).json()["ok"]
print(ev["pass"], "pass,", ev["fail"], "fail")
```

## Tests

```bash
cargo test -p forge-server
```

The integration tests in `tests/api.rs` drive the router in-process with
`tower::ServiceExt::oneshot` — health, method calls, error codes, body limit,
CORS preflight, static single-page-app fallback, the ngspice 501 path and a
round-trip of the fabrication package through a zip reader.

## Security note

This is a local development tool. It binds to `127.0.0.1` by default, has no
authentication and `POST /api/ngspice` runs a local program on user-supplied
input (in a temporary directory, with a timeout). Do not expose it to the
internet.
