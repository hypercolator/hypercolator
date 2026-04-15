/**
 * GitHub App authentication module.
 *
 * Generates a JWT signed with the App private key (RS256),
 * then exchanges it for a short-lived Installation Access Token.
 *
 * Required secrets:
 *   GITHUB_APP_ID             - numeric App ID from GitHub App settings
 *   GITHUB_APP_PRIVATE_KEY    - full PEM private key content
 *   GITHUB_APP_INSTALLATION_ID - legacy / fallback installation ID
 *
 * Optional env vars (preferred when set):
 *   GITHUB_APP_ORG_INSTALLATION_ID  - hypercolator org installation (push to org repos)
 *   GITHUB_APP_USER_INSTALLATION_ID - codes-son personal installation
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

function normalizePem(raw: string): string {
  let pem = raw.replace(/\\n/g, "\n").replace(/\r\n/g, "\n").trim();
  if (!pem.includes("\n")) {
    pem = pem
      .replace(/(-----BEGIN [^-]+-----)/g, "$1\n")
      .replace(/(-----END [^-]+-----)/g, "\n$1");
  }
  return pem;
}

async function tokenForInstallation(installationId: string): Promise<string> {
  const appId = process.env.GITHUB_APP_ID;
  const privateKey = process.env.GITHUB_APP_PRIVATE_KEY;

  if (!appId || !privateKey) {
    throw new Error("Missing required secrets: GITHUB_APP_ID, GITHUB_APP_PRIVATE_KEY");
  }

  const pemKey = normalizePem(privateKey);
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

/**
 * Get a token for the hypercolator org installation.
 * Use this for pushing to hypercolator/* repos.
 */
export async function getOrgInstallationToken(): Promise<string> {
  const id =
    process.env.GITHUB_APP_ORG_INSTALLATION_ID ||
    process.env.GITHUB_APP_INSTALLATION_ID;
  if (!id) throw new Error("No org installation ID found");
  return tokenForInstallation(id);
}

/**
 * Get a token for the codes-son personal installation.
 * Use this for pushing to codes-son/* repos.
 */
export async function getUserInstallationToken(): Promise<string> {
  const id =
    process.env.GITHUB_APP_USER_INSTALLATION_ID ||
    process.env.GITHUB_APP_INSTALLATION_ID;
  if (!id) throw new Error("No user installation ID found");
  return tokenForInstallation(id);
}

/**
 * Default: returns org token (hypercolator org = primary target).
 * Falls back to user token if org installation not configured.
 */
export async function getInstallationToken(): Promise<string> {
  const orgId = process.env.GITHUB_APP_ORG_INSTALLATION_ID;
  if (orgId) return tokenForInstallation(orgId);

  const fallbackId = process.env.GITHUB_APP_INSTALLATION_ID;
  if (!fallbackId) throw new Error("Missing GITHUB_APP_INSTALLATION_ID");
  return tokenForInstallation(fallbackId);
}

if (process.argv[1] && process.argv[1].endsWith("auth.ts")) {
  Promise.all([
    getOrgInstallationToken().then(t => log(`Org token OK (${t.slice(0, 8)}...)`)),
    getUserInstallationToken().then(t => log(`User token OK (${t.slice(0, 8)}...)`)),
  ]).catch((err: Error) => {
    console.error(err.message);
    process.exit(1);
  });
}
