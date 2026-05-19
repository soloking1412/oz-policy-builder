import { z } from "zod";

export const ScValIRSchema: z.ZodType<unknown> = z.lazy(() =>
  z.discriminatedUnion("type", [
    z.object({ type: z.literal("Address"), value: z.string() }),
    z.object({ type: z.literal("I128"), value: z.number() }),
    z.object({ type: z.literal("U128"), value: z.number() }),
    z.object({ type: z.literal("U64"), value: z.number() }),
    z.object({ type: z.literal("I64"), value: z.number() }),
    z.object({ type: z.literal("Bool"), value: z.boolean() }),
    z.object({ type: z.literal("Symbol"), value: z.string() }),
    z.object({ type: z.literal("Bytes"), value: z.array(z.number()) }),
    z.object({ type: z.literal("Vec"), value: z.array(ScValIRSchema) }),
    z.object({
      type: z.literal("Map"),
      value: z.array(z.tuple([ScValIRSchema, ScValIRSchema])),
    }),
    z.object({ type: z.literal("Void"), value: z.undefined().optional() }),
  ])
);

export const AuthNodeSchema: z.ZodType<unknown> = z.lazy(() =>
  z.object({
    signer: z.string(),
    contract: z.string(),
    function: z.string(),
    args: z.array(ScValIRSchema),
    sub_invocations: z.array(AuthNodeSchema),
  })
);

export const TokenFlowSchema = z.object({
  token: z.string(),
  from: z.string(),
  to: z.string(),
  amount: z.number(),
});

export const ProtocolHintSchema = z.discriminatedUnion("protocol", [
  z.object({
    protocol: z.literal("Blend"),
    pool: z.string(),
    backstop: z.string(),
  }),
  z.object({
    protocol: z.literal("Soroswap"),
    router: z.string(),
    path: z.array(z.string()),
  }),
  z.object({ protocol: z.literal("Sep41"), contract: z.string() }),
  z.object({
    protocol: z.literal("Unknown"),
    contract: z.string(),
    function: z.string(),
  }),
]);

export const TransactionIRSchema = z.object({
  tx_hash: z.string().nullable().optional(),
  source: z.string(),
  auth_roots: z.array(AuthNodeSchema),
  token_flows: z.array(TokenFlowSchema),
  detected_protocols: z.array(ProtocolHintSchema),
});

export const ArgConstraintSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("MaxAmount"),
    arg_index: z.number().int(),
    max: z.number(),
  }),
  z.object({
    kind: z.literal("ExactAddress"),
    arg_index: z.number().int(),
    address: z.string(),
  }),
  z.object({
    kind: z.literal("PathContains"),
    arg_index: z.number().int(),
    allowed: z.array(z.string()),
  }),
  z.object({
    kind: z.literal("Deadline"),
    arg_index: z.number().int(),
  }),
]);

export const AllowedCallSchema = z.object({
  contract: z.string(),
  function: z.string(),
  arg_constraints: z.array(ArgConstraintSchema),
});

export const PolicyParametersSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("SpendingLimit"),
    token: z.string(),
    limit: z.number(),
    window_ledgers: z.number().int(),
  }),
  z.object({
    kind: z.literal("SimpleThreshold"),
    threshold: z.number().int(),
  }),
  z.object({
    kind: z.literal("WeightedThreshold"),
    signers: z.array(z.tuple([z.string(), z.number().int()])),
    threshold: z.number().int(),
  }),
  z.object({ kind: z.literal("Custom") }),
]);

export const PolicySpecSchema = z.object({
  kind: z.enum(["SpendingLimit", "SimpleThreshold", "WeightedThreshold", "Custom"]),
  context_rule_id: z.string().min(1).max(64),
  allowed_contracts: z.array(z.string()),
  allowed_calls: z.array(AllowedCallSchema),
  parameters: PolicyParametersSchema,
  reasoning: z.array(z.string()),
  warnings: z.array(z.string()),
});

export const HarnessVectorSchema = z.object({
  id: z.string(),
  expected: z.enum(["Permit", "Deny"]),
  contract: z.string(),
  function: z.string(),
  args: z.array(ScValIRSchema),
});

export type TransactionIR = z.infer<typeof TransactionIRSchema>;
export type PolicySpec = z.infer<typeof PolicySpecSchema>;
export type HarnessVector = z.infer<typeof HarnessVectorSchema>;
