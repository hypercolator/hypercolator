/**
 * Fork and branch utilities.
 *
 * Forks aeyakovenko/percolator into the user's account,
 * then creates the hypercolator-feature branch on the fork.
 */

import { getInstallationToken } from "./auth.js";
import { log, warn } from "./logger.js";

const UPSTREAM_OWNER = "aeyakovenko";
const UPSTREAM_REPO = "percolator";
const FEATURE_BRANCH = "hypercolator-feature";

function headers(token: string): Record<string, string> {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "hypercolator-bot",
    "Content-Type": "application/json",
  };
}

export async function forkRepo(token: string, username: string): Promise<void> {
  log(`Forking ${UPSTREAM_OWNER}/${UPSTREAM_REPO} into ${username}...`);

  const checkRes = await fetch(
    `https://api.github.com/repos/${username}/${UPSTREAM_REPO}`,
    { headers: headers(token) }
  );

  if (checkRes.status === 200) {
    warn(`Fork ${username}/${UPSTREAM_REPO} already exists - skipping fork`);
    return;
  }

  const res = await fetch(
    `https://api.github.com/repos/${UPSTREAM_OWNER}/${UPSTREAM_REPO}/forks`,
    {
      method: "POST",
      headers: headers(token),
      body: JSON.stringify({
        organization: undefined,
        name: UPSTREAM_REPO,
        default_branch_only: false,
      }),
    }
  );

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Fork failed: ${res.status} ${body}`);
  }

  log(`Fork created at github.com/${username}/${UPSTREAM_REPO}`);
  log("Waiting 8s for fork to be ready...");
  await new Promise((r) => setTimeout(r, 8000));
}

export async function createFeatureBranch(
  token: string,
  username: string
): Promise<void> {
  const repoPath = `${username}/${UPSTREAM_REPO}`;
  log(`Creating branch ${FEATURE_BRANCH} on ${repoPath}...`);

  const branchCheckRes = await fetch(
    `https://api.github.com/repos/${repoPath}/git/refs/heads/${FEATURE_BRANCH}`,
    { headers: headers(token) }
  );

  if (branchCheckRes.status === 200) {
    warn(`Branch ${FEATURE_BRANCH} already exists - skipping`);
    return;
  }

  // Resolve the actual default branch (could be master, main, or anything else)
  const repoInfoRes = await fetch(
    `https://api.github.com/repos/${repoPath}`,
    { headers: headers(token) }
  );

  if (!repoInfoRes.ok) {
    const body = await repoInfoRes.text();
    throw new Error(`Could not get repo info: ${repoInfoRes.status} ${body}`);
  }

  const repoInfo = (await repoInfoRes.json()) as { default_branch: string };
  const defaultBranch = repoInfo.default_branch;
  log(`Default branch: ${defaultBranch}`);

  const defaultBranchRes = await fetch(
    `https://api.github.com/repos/${repoPath}/git/refs/heads/${defaultBranch}`,
    { headers: headers(token) }
  );

  if (!defaultBranchRes.ok) {
    const body = await defaultBranchRes.text();
    throw new Error(
      `Could not get ${defaultBranch} ref: ${defaultBranchRes.status} ${body}`
    );
  }

  const defaultBranchData = (await defaultBranchRes.json()) as {
    object: { sha: string };
  };
  const sha = defaultBranchData.object.sha;
  log(`${defaultBranch} SHA: ${sha}`);

  const createRes = await fetch(
    `https://api.github.com/repos/${repoPath}/git/refs`,
    {
      method: "POST",
      headers: headers(token),
      body: JSON.stringify({
        ref: `refs/heads/${FEATURE_BRANCH}`,
        sha,
      }),
    }
  );

  if (!createRes.ok) {
    const body = await createRes.text();
    throw new Error(`Branch creation failed: ${createRes.status} ${body}`);
  }

  log(`Branch ${FEATURE_BRANCH} created on ${repoPath}`);
}
