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
otelcol.on('close', (code) => {
  console.log(`otelcol exited with ${code}`);
});

// Wait for output to be delivered
async function expectOutput(fragment, label) {
  const deadline = Date.now() + 10000; // 10s

  while (!output.match(fragment)) {
    if (Date.now() > deadline) {
      throw new Error(`otelcol output did not contain the expected fragment '${fragment}' from ${label}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

wasm.setup();

try {
    let jsonFragment = await wasm.http_json();
    let protoFragment = await wasm.http_proto();

    await expectOutput(jsonFragment, "HTTP+JSON");
    await expectOutput(protoFragment, "HTTP+protobuf");
}
finally {
    otelcol.kill();
}
