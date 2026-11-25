import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { createReadStream, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { createHash, randomUUID, timingSafeEqual } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dataDir = process.env.DATA_DIR || join(root, "worker-data");
const mediaDir = join(dataDir, "media");
const jobDir = join(dataDir, "jobs");
mkdirSync(mediaDir, { recursive: true });
mkdirSync(jobDir, { recursive: true });
const port = Number(process.env.PORT || 8787);
const host = process.env.HOST || "127.0.0.1";
const token = process.env.WORKER_TOKEN;
const baseUrl = (process.env.PUBLIC_BASE_URL || `http://127.0.0.1:${port}`).replace(/\/$/, "");
const maxStoredRenderBytes = Number(process.env.MAX_STORED_RENDER_BYTES || 400_000_000);
if (!token || token.length < 32) throw Error("WORKER_TOKEN must be a random secret of at least 32 characters");
if (host !== "127.0.0.1" && !baseUrl.startsWith("https://")) throw Error("Public workers require an HTTPS PUBLIC_BASE_URL");
if (process.env.ENABLE_FISH_NARRATION === "1" && !process.env.FISH_API_KEY) throw Error("FISH_API_KEY is required when narration is enabled");
if (!Number.isSafeInteger(maxStoredRenderBytes) || maxStoredRenderBytes < 1) throw Error("MAX_STORED_RENDER_BYTES must be a positive integer");

const storedMediaBytes = () => readdirSync(mediaDir).reduce((total, name) => {
  const file = join(mediaDir, name);
  return total + statSync(file).size;
}, 0);
const removeIfPresent = (file) => { if (existsSync(file)) unlinkSync(file); };

const jobs = new Map();
const jobsByClientId = new Map();
const queue = [];
let rendering = false;
for (const file of (await import("node:fs")).readdirSync(jobDir)) {
  if (!file.endsWith(".json")) continue;
  try {
    const job = JSON.parse(readFileSync(join(jobDir, file), "utf8"));
    if (job.status === "rendering" || job.status === "queued") {
      job.status = "failed"; job.error = "Worker restarted before this render completed";
    }
    jobs.set(job.id, job);
    if (job.jobId && job.requestHash) jobsByClientId.set(job.jobId, job);
  } catch { /* Ignore damaged job metadata. */ }
}

const save = (job) => {
  job.updatedAt = new Date().toISOString();
  writeFileSync(join(jobDir, `${job.id}.json`), JSON.stringify(job, null, 2));
};
const respond = (res, code, data) => {
  res.writeHead(code, { "content-type": "application/json; charset=utf-8", "cache-control": "no-store", "x-content-type-options": "nosniff" });
  res.end(JSON.stringify(data));
};
const authorized = (req) => {
  const supplied = req.headers.authorization?.replace(/^Bearer /i, "") || "";
  const a = Buffer.from(supplied);
  const b = Buffer.from(token);
  const valid = a.length === b.length && timingSafeEqual(a, b);
  if (!valid && process.env.AUTH_DIAGNOSTICS === "1") {
    console.error("Authorization mismatch", { hasHeader: Boolean(req.headers.authorization), bearer: /^Bearer /i.test(req.headers.authorization || ""), suppliedLength: a.length, expectedLength: b.length });
  }
  return valid;
};
const run = (command, args) => new Promise((resolve, reject) => {
  let stderr = "";
  const child = spawn(command, args, { cwd: root, stdio: ["ignore", "ignore", "pipe"] });
  child.stderr.on("data", (chunk) => { stderr = (stderr + chunk.toString()).slice(-5000); });
  child.on("error", reject);
  child.on("close", (code, signal) => code === 0 ? resolve(stderr) : reject(Error(`${command} exited ${code ?? "without code"}${signal ? ` (${signal})` : ""}: ${stderr.slice(-700)}`)));
});
const ffprobe = async (file) => {
  const result = await new Promise((resolve, reject) => {
    let out = "";
    const child = spawn("ffprobe", ["-v", "error", "-show_entries", "stream=codec_name,width,height", "-show_entries", "format=duration", "-of", "json", file]);
    child.stdout.on("data", (chunk) => { out += chunk; });
    child.on("error", reject);
    child.on("close", (code) => code === 0 ? resolve(JSON.parse(out)) : reject(Error("ffprobe failed")));
  });
  const video = result.streams.find((stream) => stream.width);
  if (video?.codec_name !== "h264" || video.width !== 1080 || video.height !== 1920) throw Error("Output is not H.264 1080x1920");
  return Number(result.format.duration);
};
const addNarration = async (job, file, durationSeconds) => {
  if (process.env.ENABLE_FISH_NARRATION !== "1") return false;
  const script = job.inputProps.script;
  const voiceover = typeof script === "object" && script !== null ? script.voiceover : undefined;
  if (typeof voiceover !== "string" || !voiceover.trim() || voiceover.length > 5000) throw Error("Narration requires script.voiceover of 1–5000 characters");
  const response = await fetch(process.env.FISH_API_URL || "https://api.fish.audio/v1/tts", {
    method: "POST",
    headers: { Authorization: `Bearer ${process.env.FISH_API_KEY}`, "Content-Type": "application/json", model: process.env.FISH_MODEL || "s2.1-pro-free" },
    body: JSON.stringify({ text: voiceover, reference_id: process.env.FISH_VOICE_ID || "f3a2b90078d54a65af4b95f63bc7798e", format: "mp3" }),
    signal: AbortSignal.timeout(90_000),
  });
  if (!response.ok) throw Error(`Fish Audio returned HTTP ${response.status}`);
  const audio = Buffer.from(await response.arrayBuffer());
  if (audio.length < 3000 || audio.length > 20_000_000) throw Error("Fish Audio returned an invalid audio file size");
  const narrationFile = join(mediaDir, `${job.id}.narration.mp3`);
  const narratedFile = join(mediaDir, `${job.id}.narrated.mp4`);
  writeFileSync(narrationFile, audio);
  try {
    await run("ffmpeg", ["-y", "-loglevel", "error", "-i", file, "-i", narrationFile, "-filter_complex", "[1:a]loudnorm=I=-16:LRA=11:TP=-1.5,apad[a]", "-map", "0:v:0", "-map", "[a]", "-c:v", "copy", "-c:a", "aac", "-b:a", "160k", "-threads", "1", "-t", String(durationSeconds), "-movflags", "+faststart", narratedFile]);
    renameSync(narratedFile, file);
  } finally {
    if (existsSync(narrationFile)) unlinkSync(narrationFile);
    if (existsSync(narratedFile)) unlinkSync(narratedFile);
  }
  return true;
};
const pump = async () => {
  if (rendering || queue.length === 0) return;
  rendering = true;
  const job = queue.shift();
  job.status = "rendering"; save(job);
  const file = join(mediaDir, `${job.id}.mp4`);
  const propsFile = join(jobDir, `${job.id}.props.json`);
  const lowFile = join(mediaDir, `${job.id}.low.mp4`);
  try {
    if (storedMediaBytes() >= maxStoredRenderBytes) throw Error("Render storage is full; archive or remove completed media before retrying");
    writeFileSync(propsFile, JSON.stringify(job.inputProps));
    const scale = Number(process.env.RENDER_SCALE || "1");
    if (![0.5, 1].includes(scale)) throw Error("RENDER_SCALE must be 0.5 or 1");
    const renderFile = scale === 1 ? file : lowFile;
    const renderArgs = ["render", process.env.REMOTION_BUNDLE || "src/index.ts", job.composition, renderFile, "--props", propsFile, "--codec", "h264", "--concurrency", process.env.RENDER_CONCURRENCY || "1", "--scale", String(scale), "--x264-preset", "fast", "--log", "error"];
    if (process.env.RENDER_LOW_MEMORY === "1") renderArgs.push("--disallow-parallel-encoding");
    await run(join(root, "node_modules/.bin/remotion"), renderArgs);
    if (scale !== 1) await run("ffmpeg", ["-y", "-loglevel", "error", "-filter_threads", "1", "-i", renderFile, "-vf", "scale=1080:1920:flags=bicubic", "-c:v", "libx264", "-threads", "1", "-preset", "veryfast", "-crf", "18", "-movflags", "+faststart", file]);
    const durationSeconds = await ffprobe(file);
    const narrated = await addNarration(job, file, durationSeconds);
    if (narrated) await ffprobe(file);
    job.status = "completed";
    job.output = { url: `${baseUrl}/media/${job.id}.mp4`, width: 1080, height: 1920, format: "mp4", durationSeconds, narrated };
    save(job);
  } catch (error) {
    job.status = "failed"; job.error = String(error.message || error).slice(0, 800); save(job);
    removeIfPresent(file);
  } finally {
    removeIfPresent(propsFile);
    removeIfPresent(lowFile);
    rendering = false;
    void pump();
  }
};

createServer(async (req, res) => {
  const path = new URL(req.url, baseUrl).pathname;
  if (req.method === "GET" && path === "/health") return respond(res, 200, { ok: true, queueDepth: queue.length, rendering });
  const media = /^\/media\/([a-f0-9-]{36})\.mp4$/.exec(path);
  if ((req.method === "GET" || req.method === "HEAD") && media) {
    const file = join(mediaDir, `${media[1]}.mp4`);
    if (!existsSync(file)) return respond(res, 404, { error: "Not found" });
    const size = statSync(file).size;
    const common = { "content-type": "video/mp4", "cache-control": "public, max-age=86400", "accept-ranges": "bytes", "x-content-type-options": "nosniff" };
    const range = /^bytes=(\d+)-(\d*)$/.exec(req.headers.range || "");
    if (req.headers.range && !range) {
      res.writeHead(416, { ...common, "content-range": `bytes */${size}` }); return res.end();
    }
    if (range) {
      const start = Number(range[1]);
      const end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
      if (start >= size || end < start) {
        res.writeHead(416, { ...common, "content-range": `bytes */${size}` }); return res.end();
      }
      res.writeHead(206, { ...common, "content-length": end - start + 1, "content-range": `bytes ${start}-${end}/${size}` });
      return req.method === "HEAD" ? res.end() : createReadStream(file, { start, end }).pipe(res);
    }
    res.writeHead(200, { ...common, "content-length": size });
    return req.method === "HEAD" ? res.end() : createReadStream(file).pipe(res);
  }
  if (!authorized(req)) return respond(res, 401, { error: "Unauthorized" });
  if (req.method === "POST" && path === "/renders") {
    let body = "";
    try {
      for await (const chunk of req) {
        body += chunk;
        if (body.length > 100_000) return respond(res, 413, { error: "Payload too large" });
      }
      const request = JSON.parse(body);
      if (!["CreatorVertical", "BooklandMusicReal"].includes(request.composition)) return respond(res, 400, { error: "Unsupported composition" });
      if (request.output?.format !== "mp4" || request.output?.width !== 1080 || request.output?.height !== 1920) return respond(res, 400, { error: "Expected 1080x1920 MP4" });
      if (!request.inputProps || typeof request.inputProps !== "object" || Array.isArray(request.inputProps)) return respond(res, 400, { error: "inputProps must be an object" });
      if (request.composition === "BooklandMusicReal" && request.inputProps.feature !== "music") {
        return respond(res, 400, { error: "BooklandMusicReal requires feature=music" });
      }
      const jobId = String(request.jobId || "");
      const requestHash = createHash("sha256").update(JSON.stringify({ composition: request.composition, inputProps: request.inputProps, output: request.output })).digest("hex");
      const existing = jobId && jobsByClientId.get(jobId);
      if (existing) {
        if (existing.requestHash !== requestHash) return respond(res, 409, { error: "Job ID was already used with different input" });
        return respond(res, 200, { id: existing.id, renderId: existing.id, status: existing.status });
      }
      if (storedMediaBytes() >= maxStoredRenderBytes) return respond(res, 507, { error: "Render storage is full; archive or remove completed media before retrying" });
      if (queue.length >= 3) return respond(res, 429, { error: "Render queue is full" });
      const id = randomUUID();
      const job = { id, renderId: id, jobId, requestHash, composition: request.composition, status: "queued", inputProps: request.inputProps, createdAt: new Date().toISOString() };
      jobs.set(id, job); if (jobId) jobsByClientId.set(jobId, job); queue.push(job); save(job); void pump();
      return respond(res, 202, { id, renderId: id, status: job.status });
    } catch { return respond(res, 400, { error: "Invalid JSON payload" }); }
  }
  const match = /^\/renders\/([a-f0-9-]{36})$/.exec(path);
  if (req.method === "GET" && match) {
    const job = jobs.get(match[1]);
    return job ? respond(res, 200, { id: job.id, renderId: job.id, status: job.status, ...(job.output ? { output: job.output } : {}), ...(job.error ? { error: job.error } : {}) }) : respond(res, 404, { error: "Render not found" });
  }
  return respond(res, 404, { error: "Not found" });
}).listen(port, host, () => console.log(`CreatorOps worker listening on ${host}:${port}`));
