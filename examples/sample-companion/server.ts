// Nicle Dev Companion Service
// Supervised by Nicle IDE process manager (lifecycle, port management, logs, readiness check)

const port = Number(process.env.PORT) || 3050;
const hostname = "127.0.0.1";
const startTime = Date.now();

console.log(`[COMPANION] Initializing Dev Companion on http://${hostname}:${port}`);
console.log(`[COMPANION] Runtime: Bun ${Bun.version} (${process.platform} ${process.arch})`);

Bun.serve({
  port,
  hostname,
  fetch(req) {
    const url = new URL(req.url);

    // Readiness probe endpoint polled by Nicle during service startup
    if (url.pathname === "/health") {
      return new Response(
        JSON.stringify({
          status: "healthy",
          uptimeSeconds: Math.floor((Date.now() - startTime) / 1000),
          bunVersion: Bun.version,
          port,
        }),
        {
          headers: { "Content-Type": "application/json" },
        }
      );
    }

    // Metrics endpoint
    if (url.pathname === "/metrics") {
      const memory = process.memoryUsage();
      return new Response(
        JSON.stringify({
          rssBytes: memory.rss,
          heapTotalBytes: memory.heapTotal,
          heapUsedBytes: memory.heapUsed,
          pid: process.pid,
          uptime: process.uptime(),
        }),
        {
          headers: { "Content-Type": "application/json" },
        }
      );
    }

    // Web Dashboard endpoint
    return new Response(
      `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Dev Companion Dashboard</title>
  <style>
    body {
      background: #0f1115;
      color: #e6edf3;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      margin: 0;
      padding: 2.5rem;
    }
    .card {
      background: #17191e;
      border: 1px solid #232730;
      border-radius: 8px;
      padding: 1.5rem;
      max-width: 600px;
    }
    h1 {
      color: #22d3ee;
      margin-top: 0;
      font-size: 1.5rem;
    }
    .badge {
      display: inline-block;
      padding: 0.2rem 0.6rem;
      border-radius: 4px;
      font-size: 0.75rem;
      background: #102a3a;
      color: #38bdf8;
      font-weight: 600;
    }
    code {
      background: #0d1117;
      padding: 0.2rem 0.4rem;
      border-radius: 4px;
      font-family: monospace;
      color: #38bdf8;
    }
  </style>
</head>
<body>
  <div class="card">
    <div style="display: flex; justify-content: space-between; align-items: center;">
      <h1>Nicle Companion Dashboard</h1>
      <span class="badge">RUNNING</span>
    </div>
    <p>Supervised locally by Nicle process manager on <code>http://${hostname}:${port}</code>.</p>
    <ul>
      <li>Bun Version: <strong>${Bun.version}</strong></li>
      <li>PID: <strong>${process.pid}</strong></li>
      <li>Uptime: <strong>${Math.floor((Date.now() - startTime) / 1000)}s</strong></li>
    </ul>
    <p><a href="/health" style="color:#38bdf8;">View /health probe</a> | <a href="/metrics" style="color:#38bdf8;">View /metrics probe</a></p>
  </div>
</body>
</html>`,
      {
        headers: { "Content-Type": "text/html" },
      }
    );
  },
});

console.log(`[COMPANION] Ready and listening for connections`);
