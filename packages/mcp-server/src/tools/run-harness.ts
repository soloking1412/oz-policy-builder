import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { ZodTypeAny } from "zod";

import { CoreBridge } from "../bridge/mod.js";
import { PolicySpecSchema, HarnessVectorSchema } from "../types.js";

interface HarnessReport {
  results: Array<{
    vector_id: string;
    expected: "Permit" | "Deny";
    actual: "Permit" | "Deny";
    passed: boolean;
    failure_reason: string | null;
  }>;
  passed: number;
  failed: number;
}

export function registerRunHarness(server: McpServer, bridge: CoreBridge) {
  // @ts-ignore: TS2589 – MCP SDK 1.29 registerTool generic hits recursion limit with Zod v3 schemas
  server.registerTool(
    "run_harness",
    {
      description:
        "Run permit/deny test vectors against a PolicySpec. Auto-generates vectors from the spec; optionally accepts additional custom vectors.",
      inputSchema: {
        spec: PolicySpecSchema,
        vectors: z
          .object({
            permit: z.array(HarnessVectorSchema),
            deny: z.array(HarnessVectorSchema),
          })
          .optional()
          .describe("Additional custom vectors beyond the auto-generated set"),
      } as Record<string, ZodTypeAny>,
    },
    async ({ spec, vectors }) => {
      let report: HarnessReport;
      try {
        report = await bridge.call<HarnessReport>({
          method: "run_harness",
          params: {
            spec,
            extra_permit: vectors?.permit ?? [],
            extra_deny: vectors?.deny ?? [],
          },
        });
      } catch (err) {
        return {
          isError: true,
          content: [
            { type: "text" as const, text: `Harness failed: ${String(err)}` },
          ],
        };
      }

      const failedVectors = report.results.filter((r) => !r.passed);
      const status = report.failed === 0 ? "PASS" : "FAIL";
      const summary =
        `${status}: ${report.passed}/${report.passed + report.failed} vectors passed` +
        (failedVectors.length > 0
          ? `\n\nFailed vectors:\n${failedVectors
              .map((v) => `  - ${v.vector_id}: ${v.failure_reason}`)
              .join("\n")}`
          : "");

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                summary,
                total: report.passed + report.failed,
                passed: report.passed,
                failed: report.failed,
                results: report.results,
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
