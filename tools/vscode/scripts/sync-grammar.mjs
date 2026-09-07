// the "true"  grammar is in the highlighter tools folder
import { copyFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const src = resolve(here, "../../highlighter/mimas.tmLanguage.json");
const dst = resolve(here, "../syntaxes/mimas.tmLanguage.json");
copyFileSync(src, dst);
console.log(`synced grammar: ${src} -> ${dst}`);
