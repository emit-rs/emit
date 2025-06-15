const { spawn } = require('node:child_process');

const wasm = require("./native/pkg/emit_otlp_test_web_native.js");

// Spawn a collector
const otelcol = spawn('otelcol', ['--config', './config.yaml']);

let output = "";
otelcol.stdout.on('data', (data) => {
  output += data;
  console.log(output);
});
otelcol.stderr.on('data', (data) => {
  output += data;
  console.log(output);
});

wasm.setup();

// Run the integration test
let jsonFragment = wasm.http_json();
let protoFragment = wasm.http_proto();

// Wait a bit then shut down
setTimeout(() => {
    if (!output.match(jsonFragment)) {
        throw new Error(`otelcol output did not contain the expected fragment '${jsonFragment}' from HTTP+JSON`);
    }

    if (!output.match(protoFragment)) {
        throw new Error(`otelcol output did not contain the expected fragment '${protoFragment}' from HTTP+protobuf`);
    }

    otelcol.kill();
}, 1000);
