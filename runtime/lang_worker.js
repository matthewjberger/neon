// Bootstraps the Rust language worker. Buffers any message that arrives before
// the wasm has installed its real handler, then replays them once init resolves.
import init from "./lang.js";

const buffered = [];
self.onmessage = (event) => buffered.push(event);

init().then(() => {
  for (const event of buffered) {
    self.onmessage(event);
  }
});
