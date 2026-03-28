export default {
  async fetch(request, env) {
    const corsHeaders = {
      "Access-Control-Allow-Origin": "tauri://localhost",
      "Access-Control-Allow-Methods": "POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
    };

    if (request.method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders });
    }

    if (request.method !== "POST") {
      return respond(405, { error: "Method not allowed" }, corsHeaders);
    }

    let payload;
    try {
      payload = await request.json();
    } catch {
      return respond(400, { error: "Invalid JSON" }, corsHeaders);
    }

    const { type, title, body, steps, systemInfo } = payload;

    if (!["bug", "feature", "feedback"].includes(type)) {
      return respond(400, { error: "Invalid feedback type" }, corsHeaders);
    }
    if (!body || typeof body !== "string" || !body.trim()) {
      return respond(400, { error: "Body is required" }, corsHeaders);
    }

    const issueTitle = (title && title.trim()) || autoTitle(type, body);
    const issueBody = formatBody(type, body, steps, systemInfo);
    const labels = labelFor(type);

    const ghResponse = await fetch(
      `https://api.github.com/repos/${env.GITHUB_REPO}/issues`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${env.GITHUB_TOKEN}`,
          "Content-Type": "application/json",
          "User-Agent": "tables-feedback-worker",
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
        body: JSON.stringify({ title: issueTitle, body: issueBody, labels }),
      }
    );

    if (!ghResponse.ok) {
      const errText = await ghResponse.text();
      console.error("GitHub API error:", ghResponse.status, errText);
      return respond(500, { error: "Submission failed. Try again later." }, corsHeaders);
    }

    const issue = await ghResponse.json();
    return respond(200, { issue_url: issue.html_url }, corsHeaders);
  },
};

function respond(status, body, corsHeaders) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...corsHeaders },
  });
}

function autoTitle(type, body) {
  const prefix =
    type === "bug" ? "[Bug]" : type === "feature" ? "[Feature]" : "[Feedback]";
  const snippet = body.trim().slice(0, 60);
  return `${prefix} ${snippet}${body.trim().length > 60 ? "…" : ""}`;
}

function labelFor(type) {
  if (type === "bug") return ["bug"];
  if (type === "feature") return ["enhancement"];
  return ["feedback"];
}

function formatBody(type, body, steps, systemInfo) {
  if (type === "bug") {
    let md = `## What happened\n\n${body.trim()}`;
    md += `\n\n## Steps to reproduce\n\n${steps && steps.trim() ? steps.trim() : "_Not provided_"}`;
    if (systemInfo) {
      md += `\n\n---\n**System info**\n| Field | Value |\n|-------|-------|\n`;
      md += `| Version | ${systemInfo.version} |\n`;
      md += `| OS | ${systemInfo.os} |\n`;
      md += `| Arch | ${systemInfo.arch} |\n`;
      md += `| Memory | ${systemInfo.memory_gb} GB |`;
    }
    return md;
  }
  if (type === "feature") {
    return `## Why would this be useful?\n\n${body.trim()}`;
  }
  return body.trim();
}
