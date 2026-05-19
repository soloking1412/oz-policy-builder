import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

type CoreMethod = "analyze" | "synthesize" | "generate" | "run_harness";

interface CoreRequest {
  method: CoreMethod;
  params: unknown;
}

type CoreResponse<T> =
  | { ok: true; data: T }
  | { ok: false; error: string; code: string };

const TIMEOUT_MS = 30_000;

export class CoreBridge {
  constructor(private readonly binPath: string) {}

  async call<T>(req: CoreRequest): Promise<T> {
    return new Promise((resolve, reject) => {
      const child = spawn(this.binPath, [], {
        stdio: ["pipe", "pipe", "pipe"],
      });

      const timer = setTimeout(() => {
        child.kill();
        reject(new Error(`oz-policy-core timed out after ${TIMEOUT_MS}ms`));
      }, TIMEOUT_MS);

      const rl = createInterface({ input: child.stdout });
      let resolved = false;

      rl.once("line", (line) => {
        clearTimeout(timer);
        resolved = true;

        let resp: CoreResponse<T>;
        try {
          resp = JSON.parse(line) as CoreResponse<T>;
        } catch {
          reject(new Error(`oz-policy-core returned invalid JSON: ${line}`));
          return;
        }

        if (resp.ok) {
          resolve(resp.data);
        } else {
          reject(
            Object.assign(new Error(resp.error), { code: resp.code })
          );
        }
      });

      let stderr = "";
      child.stderr.on("data", (chunk: Buffer) => {
        stderr += chunk.toString();
      });

      child.on("close", (code) => {
        clearTimeout(timer);
        if (!resolved) {
          reject(
            new Error(
              `oz-policy-core exited with code ${code}${stderr ? `: ${stderr}` : ""}`
            )
          );
        }
      });

      const payload = JSON.stringify(req) + "\n";
      child.stdin.write(payload, (err) => {
        if (err) {
          clearTimeout(timer);
          reject(err);
        }
        child.stdin.end();
      });
    });
  }
}
