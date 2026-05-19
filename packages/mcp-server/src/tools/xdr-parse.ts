import { xdr, StrKey } from "@stellar/stellar-sdk";

export interface AuthNode {
  signer: string;
  contract: string;
  function: string;
  args: ScValIR[];
  sub_invocations: AuthNode[];
}

export interface TokenFlow {
  token: string;
  from: string;
  to: string;
  amount: number;
}

type ScValIR =
  | { type: "Address"; value: string }
  | { type: "I128"; value: number }
  | { type: "U128"; value: number }
  | { type: "U64"; value: number }
  | { type: "I64"; value: number }
  | { type: "Bool"; value: boolean }
  | { type: "Symbol"; value: string }
  | { type: "Bytes"; value: number[] }
  | { type: "Vec"; value: ScValIR[] }
  | { type: "Map"; value: [ScValIR, ScValIR][] }
  | { type: "Void" };

export function xdrAuthToNode(entry: xdr.SorobanAuthorizationEntry): AuthNode | null {
  try {
    const signer = extractSigner(entry.credentials());
    return parseInvocation(signer, entry.rootInvocation());
  } catch {
    return null;
  }
}

function extractSigner(creds: xdr.SorobanCredentials): string {
  const type_ = creds.switch();
  if (type_ === xdr.SorobanCredentialsType.sorobanCredentialsAddress()) {
    const addr = creds.address().address();
    return scAddressToString(addr);
  }
  return "source";
}

function parseInvocation(signer: string, inv: xdr.SorobanAuthorizedInvocation): AuthNode | null {
  const fn_ = inv.function();
  const type_ = fn_.switch();

  if (type_ !== xdr.SorobanAuthorizedFunctionType.sorobanAuthorizedFunctionTypeContractFn()) {
    return null;
  }

  const contractFn = fn_.contractFn();
  const contract = scAddressToString(contractFn.contractAddress());
  const args = contractFn.args().map(scValToIR);

  const sub_invocations: AuthNode[] = [];
  for (const sub of inv.subInvocations()) {
    const node = parseInvocation(signer, sub);
    if (node) sub_invocations.push(node);
  }

  return { signer, contract, function: contractFn.functionName().toString(), args, sub_invocations };
}

function scAddressToString(addr: xdr.ScAddress): string {
  const type_ = addr.switch();
  if (type_ === xdr.ScAddressType.scAddressTypeAccount()) {
    const pk = addr.accountId().ed25519();
    return StrKey.encodeEd25519PublicKey(pk);
  }
  if (type_ === xdr.ScAddressType.scAddressTypeContract()) {
    return StrKey.encodeContract(addr.contractId());
  }
  return "unknown";
}

function scValToIR(val: xdr.ScVal): ScValIR {
  const type_ = val.switch();

  if (type_ === xdr.ScValType.scvAddress()) {
    return { type: "Address", value: scAddressToString(val.address()) };
  }
  if (type_ === xdr.ScValType.scvI128()) {
    const parts = val.i128();
    const hi = BigInt(parts.hi().toString());
    const lo = BigInt(parts.lo().toString());
    const v = Number((hi << 64n) | lo);
    return { type: "I128", value: v };
  }
  if (type_ === xdr.ScValType.scvU128()) {
    const parts = val.u128();
    const hi = BigInt(parts.hi().toString());
    const lo = BigInt(parts.lo().toString());
    const v = Number((hi << 64n) | lo);
    return { type: "U128", value: v };
  }
  if (type_ === xdr.ScValType.scvU64()) {
    return { type: "U64", value: Number(val.u64().toString()) };
  }
  if (type_ === xdr.ScValType.scvI64()) {
    return { type: "I64", value: Number(val.i64().toString()) };
  }
  if (type_ === xdr.ScValType.scvBool()) {
    return { type: "Bool", value: val.b() };
  }
  if (type_ === xdr.ScValType.scvSymbol()) {
    return { type: "Symbol", value: val.sym().toString() };
  }
  if (type_ === xdr.ScValType.scvBytes()) {
    return { type: "Bytes", value: Array.from(val.bytes()) };
  }
  if (type_ === xdr.ScValType.scvVec()) {
    const items = val.vec() ?? [];
    return { type: "Vec", value: items.map(scValToIR) };
  }
  if (type_ === xdr.ScValType.scvMap()) {
    const entries = val.map() ?? [];
    return {
      type: "Map",
      value: entries.map((e) => [scValToIR(e.key()), scValToIR(e.val())] as [ScValIR, ScValIR]),
    };
  }

  return { type: "Void" };
}

export function xdrDiagnosticToFlow(event: xdr.DiagnosticEvent): TokenFlow | null {
  try {
    const contractId = event.event().contractId();
    if (!contractId) return null;

    const token = StrKey.encodeContract(contractId);
    const body = event.event().body();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const eventType = body.switch() as any;
    const expected = xdr.ContractEventType.contract() as unknown as { value: number };
    const actual = typeof eventType === "number" ? eventType : (eventType as { value: number }).value;
    if (actual !== expected.value) return null;

    const v0 = body.v0();
    const topics = v0.topics();
    if (topics.length < 3) return null;

    const topic0 = topics[0];
    if (topic0.switch() !== xdr.ScValType.scvSymbol()) return null;
    if (topic0.sym().toString() !== "transfer") return null;

    const fromIR = scValToIR(topics[1]);
    const toIR = scValToIR(topics[2]);
    if (fromIR.type !== "Address" || toIR.type !== "Address") return null;

    const amountIR = scValToIR(v0.data());
    let amount = 0;
    if (amountIR.type === "I128" || amountIR.type === "U128") {
      amount = amountIR.value;
    }

    return { token, from: fromIR.value, to: toIR.value, amount };
  } catch {
    return null;
  }
}
