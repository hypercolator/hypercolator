/**
 * GitHub App authentication module.
 *
 * Generates a JWT signed with the App private key (RS256),
 * then exchanges it for a short-lived Installation Access Token.
 *
 * Required env vars:
 *   GITHUB_APP_ID             - numeric App ID from GitHub App settings
 *   GITHUB_APP_PRIVATE_KEY    - full PEM private key content
 *   GITHUB_APP_INSTALLATION_ID - installation ID (from /settings/installations/...)
 */

import { createSign } from "crypto";
import { log } from "./logger.js";

function base64url(input: Buffer | string): string {
  const buf = typeof input === "string" ? Buffer.from(input) : input;
  return buf
    .toString("base64")
    .replace(/=/g, "")
    .replace(/\+/g, "-")
    .replace(/\//g, "_");
}

function buildJwt(appId: string, privateKeyPem: string): string {
  const now = Math.floor(Date.now() / 1000);
  const header = base64url(JSON.stringify({ alg: "RS256", typ: "JWT" }));
  const payload = base64url(
    JSON.stringify({
      iat: now - 60,
      exp: now + 600,
      iss: appId,
    })
  );
  const signingInput = `${header}.${payload}`;
  const sign = createSign("RSA-SHA256");
  sign.update(signingInput);
  const sig = sign.sign(privateKeyPem, "base64");
  const sigUrl = sig
    .replace(/=/g, "")
    .replace(/\+/g, "-")
    .replace(/\//g, "_");
  return `${signingInput}.${sigUrl}`;
}

export async function getInstallationToken(): Promise<string> {
  const appId = process.env.GITHUB_APP_ID;
  const privateKey = process.env.GITHUB_APP_PRIVATE_KEY;
  const installationId = process.env.GITHUB_APP_INSTALLATION_ID;

  if (!appId || !privateKey || !installationId) {
    throw new Error(
      "Missing required secrets: GITHUB_APP_ID, GITHUB_APP_PRIVATE_KEY, GITHUB_APP_INSTALLATION_ID"
    );
  }

  // Normalize PEM: handle literal \n sequences, Windows line endings,
  // and ensure proper PEM block structure
  let pemKey = privateKey
    .replace(/\\n/g, "\n")
    .replace(/\r\n/g, "\n")
    .trim();

  // If the PEM header/footer are on the same line as content, split them
  if (!pemKey.includes("\n")) {
    pemKey = pemKey
      .replace(/(-----BEGIN [^-]+-----)/g, "$1\n")
      .replace(/(-----END [^-]+-----)/g, "\n$1");
  }
  const jwt = buildJwt(appId, pemKey);
  log("Generated GitHub App JWT");

  const url = `https://api.github.com/app/installations/${installationId}/access_tokens`;
  const res = await fetch(url, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${jwt}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      "User-Agent": "hypercolator-bot",
    },
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Failed to get installation token: ${res.status} ${body}`);
  }

  const data = (await res.json()) as { token: string; expires_at: string };
  log(`Installation token obtained - expires at ${data.expires_at}`);
  return data.token;
}

if (process.argv[1] && process.argv[1].endsWith("auth.ts")) {
  getInstallationToken()
    .then((tok) => {
      log(`Token starts with: ${tok.slice(0, 12)}...`);
      log("Auth OK");
    })
    .catch((err: Error) => {
      console.error(err.message);
      process.exit(1);
    });
}
