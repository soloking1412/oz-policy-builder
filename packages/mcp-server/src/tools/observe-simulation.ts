import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { rpc, xdr } from "@stellar/stellar-sdk";
import { z } from "zod";
import type { ZodTypeAny } from "zod";

import { CoreBridge } from "../bridge/mod.js";
import { RPC_URLS } from "../rpc.js";
import type { TransactionIR } from "../types.js";
import { xdrAuthToNode, xdrDiagnosticToFlow } from "./xdr-parse.js";

export function registerObserveSimulation(server: McpServer, bridge: CoreBridge) {
  // @ts-ignore: TS2589 – MCP SDK 1.29 registerTool generic hits recursion limit with Zod v3 schemas
  server.registerTool(
    "observe_simulation",
    {
      description:
        "Simulate a transaction XDR and extract a structured IR without submitting to the network",
      inputSchema: {
        xdr: z.string().describe("Base64 XDR of a transaction envelope to simulate"),
        network: z.enum(["mainnet", "testnet", "futurenet"]).default("testnet"),
        rpc_url: z.string().url().optional(),
      } as Record<string, ZodTypeAny>,
    },
    async ({ xdr: txXdr, network, rpc_url }) => {
      const rpcUrl = rpc_url ?? process.env["STELLAR_RPC_URL"] ?? RPC_URLS[network];
      const sorobanServer = new rpc.Server(rpcUrl);

      let txEnv: xdr.TransactionEnvelope;
      let simResult: rpc.Api.SimulateTransactionResponse;
      try {
        txEnv = xdr.TransactionEnvelope.fromXDR(txXdr, "base64");
        simResult = await sorobanServer.simulateTransaction(
          txEnv as unknown as Parameters<
            typeof sorobanServer.simulateTransaction
          >[0]
        );
      } catch (err) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `Simulation error: ${String(err)}` }],
        };
      }

      if (rpc.Api.isSimulationError(simResult)) {
        return {
          isError: true,
          content: [
            {
              type: "text" as const,
              text: `Simulation failed: ${(simResult as { error: string }).error}`,
            },
          ],
        };
      }

      const authRoots: unknown[] = [];
      const tokenFlows: unknown[] = [];

      if (rpc.Api.isSimulationSuccess(simResult) && simResult.result?.auth) {
        for (const authEntry of simResult.result
          .auth as xdr.SorobanAuthorizationEntry[]) {
          const node = xdrAuthToNode(authEntry);
          if (node) authRoots.push(node);
        }
      }

      for (const ev of (simResult.events ?? []) as xdr.DiagnosticEvent[]) {
        const flow = xdrDiagnosticToFlow(ev);
        if (flow) tokenFlows.push(flow);
      }

      const sourceAccount =
        (
          txEnv as unknown as {
            tx?: { sourceAccount?: { toString?: () => string } };
          }
        ).tx?.sourceAccount?.toString?.() ?? "";

      const ir = await bridge.call<TransactionIR>({
        method: "analyze",
        params: {
          input: {
            auth_roots: authRoots,
            token_flows: tokenFlows.length > 0 ? tokenFlows : null,
            source_account: sourceAccount,
            tx_hash: null,
          },
        },
      });

      const warnings = ir.detected_protocols
        .filter((p) => (p as { protocol: string }).protocol === "Unknown")
        .map(
          (p) =>
            `unrecognized: contract=${(p as { contract: string }).contract}`
        );

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(
              {
                ir,
                sim_result: { success: true },
                summary: `simulated | auth roots: ${authRoots.length} | flows: ${ir.token_flows.length}`,
                warnings,
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
