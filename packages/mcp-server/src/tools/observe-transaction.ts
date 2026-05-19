import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { rpc, xdr } from "@stellar/stellar-sdk";
import { z } from "zod";
import type { ZodTypeAny } from "zod";

import { CoreBridge } from "../bridge/mod.js";
import { RPC_URLS } from "../rpc.js";
import type { TransactionIR } from "../types.js";
import { xdrAuthToNode, xdrDiagnosticToFlow } from "./xdr-parse.js";

type EnvelopeParsed = {
  tx?: {
    sourceAccount?: { toString?: () => string };
    operations?: Array<{
      body?: { value?: { auth?: Array<xdr.SorobanAuthorizationEntry> } };
    }>;
  };
};

export function registerObserveTransaction(server: McpServer, bridge: CoreBridge) {
  // @ts-ignore: TS2589 – MCP SDK 1.29 registerTool generic hits recursion limit with Zod v3 schemas
  server.registerTool(
    "observe_transaction",
    {
      description:
        "Fetch a Stellar transaction by hash and extract a structured IR from its authorization entries",
      inputSchema: {
        tx_hash: z.string().describe("Stellar transaction hash"),
        network: z.enum(["mainnet", "testnet", "futurenet"]).default("mainnet"),
        rpc_url: z.string().url().optional().describe("Override RPC endpoint"),
      } as Record<string, ZodTypeAny>,
    },
    async ({ tx_hash, network, rpc_url }) => {
      const rpcUrl = rpc_url ?? process.env["STELLAR_RPC_URL"] ?? RPC_URLS[network];
      const sorobanServer = new rpc.Server(rpcUrl);

      let txResult: rpc.Api.GetTransactionResponse;
      try {
        txResult = await sorobanServer.getTransaction(tx_hash);
      } catch (err) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `RPC error: ${String(err)}` }],
        };
      }

      if (txResult.status !== rpc.Api.GetTransactionStatus.SUCCESS) {
        return {
          isError: true,
          content: [
            { type: "text" as const, text: `Transaction not successful: ${txResult.status}` },
          ],
        };
      }

      const successTx = txResult as rpc.Api.GetSuccessfulTransactionResponse;
      const envelope = successTx.envelopeXdr as unknown as EnvelopeParsed;
      const authRoots: unknown[] = [];
      const tokenFlows: unknown[] = [];

      for (const op of envelope.tx?.operations ?? []) {
        for (const authEntry of op.body?.value?.auth ?? []) {
          const node = xdrAuthToNode(authEntry);
          if (node) authRoots.push(node);
        }
      }

      if (successTx.resultMetaXdr) {
        const meta = successTx.resultMetaXdr as unknown as {
          sorobanMeta?: { diagnosticEvents?: Array<xdr.DiagnosticEvent> };
        };
        for (const ev of meta.sorobanMeta?.diagnosticEvents ?? []) {
          const flow = xdrDiagnosticToFlow(ev);
          if (flow) tokenFlows.push(flow);
        }
      }

      const sourceAccount = envelope.tx?.sourceAccount?.toString?.() ?? "";

      const ir = await bridge.call<TransactionIR>({
        method: "analyze",
        params: {
          input: {
            auth_roots: authRoots,
            token_flows: tokenFlows.length > 0 ? tokenFlows : null,
            source_account: sourceAccount,
            tx_hash,
          },
        },
      });

      const contracts = new Set(
        (ir.auth_roots as Array<{ contract: string }>).map((n) => n.contract)
      );
      const summary =
        `tx=${tx_hash} | contracts: ${contracts.size} | ` +
        `flows: ${ir.token_flows.length} | ` +
        `protocols: ${ir.detected_protocols
          .map((p) => (p as { protocol: string }).protocol)
          .join(", ")}`;

      const warnings = ir.detected_protocols
        .filter((p) => (p as { protocol: string }).protocol === "Unknown")
        .map((p) => `unrecognized: contract=${(p as { contract: string }).contract}`);

      return {
        content: [
          { type: "text" as const, text: JSON.stringify({ ir, summary, warnings }, null, 2) },
        ],
      };
    }
  );
}
