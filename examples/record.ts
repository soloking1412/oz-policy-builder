#!/usr/bin/env tsx
/**
 * Fetches a Soroban transaction by hash, pulls out its authorization entries
 * and diagnostic events, and writes them to an example's fixtures/ directory.
 *
 *   export STELLAR_RPC_URL=https://soroban-testnet.stellar.org
 *   tsx examples/record.ts <example> <transaction_hash>
 *
 * <example> is the example folder name, e.g. blend-yield-claim.
 */

import { rpc } from "@stellar/stellar-sdk";
import { writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const EXAMPLES = ["blend-yield-claim", "soroswap-bounded", "sep41-subscription"];

const examplesDir = dirname(fileURLToPath(import.meta.url));

async function main() {
  const [example, txHash] = process.argv.slice(2);

  if (!example || !txHash || !EXAMPLES.includes(example)) {
    console.error("Usage: tsx examples/record.ts <example> <transaction_hash>");
    console.error(`  <example> is one of: ${EXAMPLES.join(", ")}`);
    process.exit(1);
  }

  const rpcUrl =
    process.env["STELLAR_RPC_URL"] ?? "https://soroban-testnet.stellar.org";
  const server = new rpc.Server(rpcUrl);

  console.log(`Fetching ${txHash} from ${rpcUrl} ...`);
  const tx = await server.getTransaction(txHash);

  if (tx.status !== rpc.Api.GetTransactionStatus.SUCCESS) {
    console.error(`Transaction not successful: ${tx.status}`);
    process.exit(1);
  }

  const authEntries: string[] = [];
  const diagnosticEvents: string[] = [];

  const meta = tx.resultMetaXdr as unknown as {
    sorobanMeta?: { diagnosticEvents?: Array<{ toXDR: (f: string) => string }> };
  };
  for (const ev of meta?.sorobanMeta?.diagnosticEvents ?? []) {
    diagnosticEvents.push(ev.toXDR("base64"));
  }

  const envelope = tx.envelopeXdr as unknown as {
    tx?: {
      sourceAccount?: { toString?: () => string };
      operations?: Array<{
        body?: { value?: { auth?: Array<{ toXDR: (f: string) => string }> } };
      }>;
    };
  };

  for (const op of envelope.tx?.operations ?? []) {
    for (const entry of op.body?.value?.auth ?? []) {
      authEntries.push(entry.toXDR("base64"));
    }
  }

  const fixture = {
    source_account: envelope.tx?.sourceAccount?.toString?.() ?? "",
    tx_hash: txHash,
    auth_entries: authEntries,
    diagnostic_events: diagnosticEvents,
  };

  const outPath = resolve(examplesDir, example, "fixtures", "sim_response.json");
  writeFileSync(outPath, JSON.stringify(fixture, null, 2));

  console.log(`Wrote ${outPath}`);
  console.log(`  auth entries:      ${authEntries.length}`);
  console.log(`  diagnostic events: ${diagnosticEvents.length}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
