import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { ZodTypeAny } from "zod";

import { CoreBridge } from "../bridge/mod.js";
import { TransactionIRSchema, type PolicySpec } from "../types.js";

export function registerSynthesizePolicy(server: McpServer, bridge: CoreBridge) {
  // @ts-ignore: TS2589 – MCP SDK 1.29 registerTool generic hits recursion limit with Zod v3 schemas
  server.registerTool(
    "synthesize_policy",
    {
      description:
        "Derive the minimal OpenZeppelin context rule and policy set from a TransactionIR",
      inputSchema: {
        ir: TransactionIRSchema,
        context_rule_id: z
          .string()
          .min(1)
          .max(64)
          .describe("Identifier for the context rule (e.g. 'blend_yield_daily')"),
        overrides: z
          .object({
            force_custom: z
              .boolean()
              .default(false)
              .describe("Skip primitive selection and always generate a custom contract"),
            spending_limit_multiplier: z
              .number()
              .min(1)
              .max(10)
              .default(1.1)
              .describe("Multiply observed max amount to compute policy ceiling"),
            window_ledgers: z
              .number()
              .int()
              .min(100)
              .default(17280)
              .describe("Rolling window size in ledgers (17280 ≈ 1 day)"),
          })
          .optional(),
      } as Record<string, ZodTypeAny>,
    },
    async ({ ir, context_rule_id, overrides }) => {
      const options = {
        context_rule_id,
        force_custom: overrides?.force_custom ?? false,
        spending_limit_multiplier: overrides?.spending_limit_multiplier ?? 1.1,
        window_ledgers: overrides?.window_ledgers ?? 17280,
      };

      let spec: PolicySpec;
      try {
        spec = await bridge.call<PolicySpec>({
          method: "synthesize",
          params: { ir, options },
        });
      } catch (err) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `Synthesis failed: ${String(err)}` }],
        };
      }

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                spec,
                reasoning: spec.reasoning,
                primitive_used: spec.kind,
                warnings: spec.warnings,
              },
              null,
              2
            ),
          },
        ],
      };
    }
  );
}
