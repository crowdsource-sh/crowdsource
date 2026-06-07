import init, { Client } from "../../dist/pkg/crowdsource";

// wasm-bindgen (--target web) emits a default `init()` that loads the .wasm.
// Await it once before constructing a Client:
//
//   import init, { Client } from "@crowdsource/client";
//   await init();
//   const cs = new Client("https://api.crowdsource.sh", apiKey);
//   const { competitions } = await cs.listCompetitions("open");
export default init;

export { Client } from "../../dist/pkg/crowdsource";
export * as wasm from "../../dist/pkg/crowdsource";
