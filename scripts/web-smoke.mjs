// Smoke test for the built web app: open it in headless Chromium, draw a rectangle, pull it into a
// block, and report what happened. Run in CI after `trunk build` (see .github/workflows/ci.yml):
//   node scripts/web-smoke.mjs dist/web
// Needs: npm install playwright pngjs; npx playwright install chromium.
// It prints SMOKE lines, and the two screenshots as base64 between SHOT markers, so a person with
// only the job log can still see the app (decode with: sed -n '/SHOT_BEGIN name/,/SHOT_END/p').
import { chromium } from "playwright";
import { PNG } from "pngjs";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.argv[2] ?? "dist/web");
const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css", ".png": "image/png", ".json": "application/json" };
const server = http.createServer((req, res) => {
  let p = decodeURIComponent(new URL(req.url, "http://x").pathname);
  if (p.endsWith("/")) p += "index.html";
  const file = path.join(root, p);
  if (!file.startsWith(root)) return res.writeHead(403).end();
  fs.readFile(file, (err, body) => {
    if (err) return res.writeHead(404).end();
    res.writeHead(200, { "content-type": types[path.extname(file)] ?? "application/octet-stream" });
    res.end(body);
  });
});
await new Promise((ok) => server.listen(8080, "127.0.0.1", ok));

const problems = [];
const say = (...a) => console.log("SMOKE", ...a);

const browser = await chromium.launch({
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist", "--enable-webgl", "--use-gl=angle"],
});
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
page.on("console", (m) => {
  if (m.type() === "error") errors.push(`console.error: ${m.text()}`);
  if (m.type() === "warning" || m.type() === "info") say("console", m.type(), m.text().slice(0, 200));
});

const shot = async (name) => {
  const buf = await page.screenshot();
  fs.mkdirSync("smoke", { recursive: true });
  fs.writeFileSync(`smoke/${name}.png`, buf);
  console.log(`SHOT_BEGIN ${name}`);
  console.log(buf.toString("base64").match(/.{1,200}/g).join("\n"));
  console.log("SHOT_END");
  return PNG.sync.read(buf);
};
const colours = (img) => {
  const seen = new Set();
  for (let i = 0; i < img.data.length; i += 4) seen.add((img.data[i] << 16) | (img.data[i + 1] << 8) | img.data[i + 2]);
  return seen.size;
};
const changed = (a, b) => {
  let n = 0;
  for (let i = 0; i < a.data.length; i += 4) {
    if (Math.abs(a.data[i] - b.data[i]) + Math.abs(a.data[i + 1] - b.data[i + 1]) + Math.abs(a.data[i + 2] - b.data[i + 2]) > 30) n++;
  }
  return n;
};

await page.goto("http://127.0.0.1:8080/", { waitUntil: "load" });
say("webgpu adapter:", await page.evaluate(async () => (navigator.gpu ? !!(await navigator.gpu.requestAdapter()) : "no navigator.gpu")));

// 1. It starts: the loading text goes away once eframe has started.
try {
  await page.waitForSelector("#solidsketch_loading", { state: "detached", timeout: 90000 });
  say("started: the loading text is gone");
} catch {
  problems.push("the app did not finish starting within 90 s");
}
await page.waitForTimeout(3000);
const start = await shot("1-start");
say("start picture: distinct colours =", colours(start));
if (colours(start) < 20) problems.push("the first picture is nearly blank");

// 2. Draw a rectangle on the ground: Rectangle tool, two clicks.
await page.mouse.click(640, 400); // focus
await page.keyboard.press("r");
await page.mouse.move(540, 420);
await page.mouse.click(540, 420);
await page.mouse.move(640, 470, { steps: 5 });
await page.mouse.click(740, 520);
await page.waitForTimeout(2500);
const drawn = await shot("2-rectangle");
say("rectangle: pixels changed vs start =", changed(start, drawn));

// 3. Pull it up: Push/Pull tool, drag the pointer up.
await page.keyboard.press("p");
await page.mouse.move(640, 470);
await page.mouse.down();
await page.mouse.move(640, 430, { steps: 6 });
await page.mouse.move(640, 380, { steps: 6 });
await page.mouse.up();
await page.waitForTimeout(3000);
const pulled = await shot("3-block");
say("block: pixels changed vs rectangle =", changed(drawn, pulled));
if (changed(drawn, pulled) < 500) say("INFO the pull changed few pixels: the block may not have appeared");

if (errors.length) {
  for (const e of errors.slice(0, 20)) say("ERROR", e.slice(0, 400));
  problems.push(`${errors.length} browser error(s)`);
}
await browser.close();
server.close();
if (problems.length) {
  say("FAILED:", problems.join("; "));
  process.exit(1);
}
say("passed");
