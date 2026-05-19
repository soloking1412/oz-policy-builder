import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

import { CoreBridge } from "./bridge/mod.js";
import {
  registerObserveTransaction,
  registerObserveSimulation,
  registerSynthesizePolicy,
  registerGenerateCode,
  registerRunHarness,
} from "./tools/index.js";
import { registerPolicySchemaResource } from "./resources/policy-schema.js";
import { registerPrimitivesRefResource } from "./resources/primitives-ref.js";

export function createServer(bridge: CoreBridge): McpServer {
  const server = new McpServer({
    name: "oz-policy-builder",
    version: "0.1.0",
  });

  registerObserveTransaction(server, bridge);
  registerObserveSimulation(server, bridge);
  registerSynthesizePolicy(server, bridge);
  registerGenerateCode(server, bridge);
  registerRunHarness(server, bridge);

  registerPolicySchemaResource(server);
  registerPrimitivesRefResource(server);

  return server;
}
