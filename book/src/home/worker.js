// the repl runs here, off the page thread, so a `loop {}` can't freeze the tab
import init, { Repl } from './repl.js';

// the handler goes in before the wasm loads, so an input sent meanwhile isn't dropped
const repl = init().then(() => new Repl());

onmessage = async ({ data }) => {
  try {
    const session = await repl;
    if (Repl.unfinished(data)) return postMessage({ unfinished: true });
    const out = session.run(data);
    postMessage({ printed: out.printed, echo: out.echo, error: out.error });
    out.free();
  } catch (err) {
    // a panic leaves the wasm unusable
    postMessage({ error: `${err} (reset to start over)` });
  }
};
