import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { setTimeout as delay } from "node:timers/promises";

const token = "test-token-at-least-thirty-two-characters-long";
const workerFile = new URL("./server.mjs", import.meta.url);
const request = { jobId: "storage-limit-test", composition: "CreatorVertical", inputProps: { brand: "Bookland" }, output: { format: "mp4", width: 1080, height: 1920 } };

const freePort = async () => {
  const server = createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
};

const startWorker = async (dataDir, extraEnv = {}) => {
  const port = await freePort();
  const child = spawn(process.execPath, [workerFile.pathname], {
    env: { ...process.env, HOST: "127.0.0.1", PORT: String(port), DATA_DIR: dataDir, WORKER_TOKEN: token, PUBLIC_BASE_URL: `http://127.0.0.1:${port}`, ...extraEnv },
    stdio: "ignore",
  });
  const base = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 50; i++) {
    if (child.exitCode !== null) throw Error(`Worker exited with ${child.exitCode}`);
    try { if ((await fetch(`${base}/health`)).ok) return { child, base }; } catch { /* Still starting. */ }
    await delay(50);
  }
  child.kill();
  throw Error("Worker did not start");
};

const submit = (base, body = request) => fetch(`${base}/renders`, {
  method: "POST",
  headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
  body: JSON.stringify(body),
});

test("rejects renders at the storage limit without deleting existing media", async () => {
  const dataDir = mkdtempSync(join(tmpdir(), "creatorops-storage-"));
  const mediaDir = join(dataDir, "media");
  mkdirSync(mediaDir);
  writeFileSync(join(mediaDir, "existing.mp4"), "keep");
  const { child, base } = await startWorker(dataDir, { MAX_STORED_RENDER_BYTES: "4" });
  try {
    const response = await submit(base);
    if (response.status !== 507) throw Error(`Expected 507, got ${response.status}`);
    if (!readdirSync(mediaDir).includes("existing.mp4")) throw Error("Existing media was deleted");
  } finally {
    child.kill();
    rmSync(dataDir, { recursive: true, force: true });
  }
});

test("removes temporary props when a render fails", async () => {
  const dataDir = mkdtempSync(join(tmpdir(), "creatorops-cleanup-"));
  const { child, base } = await startWorker(dataDir, { RENDER_SCALE: "0.3" });
  try {
    const response = await submit(base);
    if (response.status !== 202) throw Error(`Expected 202, got ${response.status}`);
    const { id } = await response.json();
    let job;
    for (let i = 0; i < 50; i++) {
      job = await (await fetch(`${base}/renders/${id}`, { headers: { authorization: `Bearer ${token}` } })).json();
      if (job.status === "failed") break;
      await delay(50);
    }
    if (job?.status !== "failed") throw Error("Render did not fail");
    if (readdirSync(join(dataDir, "jobs")).some((name) => name.endsWith(".props.json"))) throw Error("Temporary props remain");
    if (readdirSync(join(dataDir, "media")).length) throw Error("Temporary media remains");
  } finally {
    child.kill();
    rmSync(dataDir, { recursive: true, force: true });
  }
});
