import { getInstallationToken } from "./auth.js";

async function main() {
  const token = await getInstallationToken();
  const res = await fetch(
    "https://api.github.com/repos/codes-son/codes-son.github.io/contents/index.html",
    {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "hypercolator-bot",
      },
    }
  );
  const data = (await res.json()) as { content: string; sha: string };
  const html = Buffer.from(data.content, "base64").toString("utf-8");
  console.log("SHA:", data.sha);
  console.log("Has Hypercolator card:", html.includes("https://github.com/hypercolator"));
  console.log("Has Bot card:", html.includes("https://github.com/apps/hypercolator-bot"));
  console.log("Has banned button:", html.includes("See "));
  // Show the projects section only
  const start = html.indexOf('<section id="projects">');
  const end = html.indexOf("</section>", start) + "</section>".length;
  console.log("\n--- Projects section ---");
  console.log(html.slice(start, end));
}

main().catch(console.error);
