import { spawn } from "node:child_process";

import * as wasm from "./native/pkg/emit_otlp_test_web_native.js";

// Spawn a collector
const otelcol = spawn("otelcol", ["--config", "./config.yaml"]);

let output = "";
otelcol.stdout.on("data", (data) => {
  output += data;
  console.log(output);
});
otelcol.stderr.on("data", (data) => {
  output += data;
  console.log(output);
});

wasm.setup();

try {
    let jsonFragment = await wasm.http_json();
    let protoFragment = await wasm.http_proto();

    if (!output.match(jsonFragment)) {
        throw new Error(`otelcol output did not contain the expected fragment '${jsonFragment}' from HTTP+JSON`);
    }

    if (!output.match(protoFragment)) {
        throw new Error(`otelcol output did not contain the expected fragment '${protoFragment}' from HTTP+protobuf`);
    }
}
finally {
    otelcol.kill();
}
