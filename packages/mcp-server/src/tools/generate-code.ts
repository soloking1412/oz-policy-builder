import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { ZodTypeAny } from "zod";

import { CoreBridge } from "../bridge/mod.js";
import { PolicySpecSchema } from "../types.js";

interface GeneratedPolicy {
  cargo_toml: string;
  lib_rs: string;
  tests_rs: string;
  warnings: string[];
}

export function registerGenerateCode(server: McpServer, bridge: CoreBridge) {
  // @ts-ignore: TS2589 – MCP SDK 1.29 registerTool generic hits recursion limit with Zod v3 schemas
  server.registerTool(
    "generate_code",
    {
      description:
        "Generate compilable Rust policy contract code from a PolicySpec. Returns Cargo.toml, src/lib.rs, and src/tests.rs. Review before deploying.",
      inputSchema: {
        spec: PolicySpecSchema,
        package_name: z
          .string()
          .regex(/^[a-z][a-z0-9-]*$/)
          .describe(
            "Rust package name for the generated crate (e.g. 'my-blend-policy')"
          ),
      } as Record<string, ZodTypeAny>,
    },
    async ({ spec, package_name }) => {
      let policy: GeneratedPolicy;
      try {
        policy = await bridge.call<GeneratedPolicy>({
          method: "generate",
          params: { spec, package_name },
        });
      } catch (err) {
        return {
          isError: true,
          content: [
            {
              type: "text" as const,
              text: `Code generation failed: ${String(err)}`,
            },
          ],
        };
      }

      const next_steps = [
        `# Audit src/lib.rs before building`,
        `cargo build --target wasm32v1-none --profile contract`,
        `cargo test  # runs permit/deny harness`,
        `stellar contract deploy --wasm target/wasm32v1-none/contract/${package_name}.wasm`,
        `stellar contract invoke --id <policy_address> -- install --account <smart_account> --rule_id <rule_id> [args...]`,
      ];

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                files: {
                  "Cargo.toml": policy.cargo_toml,
                  "src/lib.rs": policy.lib_rs,
                  "src/tests.rs": policy.tests_rs,
                },
                warnings: policy.warnings,
                next_steps,
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
