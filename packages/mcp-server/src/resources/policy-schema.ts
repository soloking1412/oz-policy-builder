import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { zodToJsonSchema } from "zod-to-json-schema";
import { TransactionIRSchema, PolicySpecSchema } from "../types.js";

export function registerPolicySchemaResource(server: McpServer) {
  server.resource(
    "policy://schema",
    "policy://schema",
    { mimeType: "application/json" },
    async () => {
      const schema = {
        TransactionIR: zodToJsonSchema(TransactionIRSchema, "TransactionIR"),
        PolicySpec: zodToJsonSchema(PolicySpecSchema, "PolicySpec"),
      };
      return {
        contents: [
          {
            uri: "policy://schema",
            mimeType: "application/json",
            text: JSON.stringify(schema, null, 2),
          },
        ],
      };
    }
  );
}
