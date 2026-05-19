import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { CoreBridge } from "./bridge/mod.js";
import { createServer } from "./server.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function resolveBinPath(): string {
  return (
    process.env["OZ_POLICY_CORE_BIN"] ??
    path.resolve(__dirname, "../../../target/release/oz-policy-core")
  );
}

async function main() {
  const binPath = resolveBinPath();
  const bridge = new CoreBridge(binPath);
  const server = createServer(bridge);

  const transport = new StdioServerTransport();
  await server.connect(transport);

  process.stderr.write(`oz-policy-builder MCP server started (core: ${binPath})\n`);
}

main().catch((err) => {
  process.stderr.write(`Fatal: ${String(err)}\n`);
  process.exit(1);
});
