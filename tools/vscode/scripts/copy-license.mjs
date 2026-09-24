// vsce wants a single LICENSE, so the repo's two licenses go in one after the other
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const mit = readFileSync(resolve(root, "LICENSE-MIT"), "utf8");
const apache = readFileSync(resolve(root, "LICENSE-APACHE"), "utf8");
const dst = resolve(root, "tools/vscode/LICENSE");
writeFileSync(dst, `mimas is dual-licensed under MIT or Apache 2.0, at your option.\n\n${mit}\n\n${apache}`);
console.log(`wrote ${dst}`);
