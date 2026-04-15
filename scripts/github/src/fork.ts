/**
 * Fork and branch utilities.
 *
 * Forks aeyakovenko/percolator into the user's account,
 * then creates the hypercolator-feature branch on the fork.
 */

import { log, warn } from "./logger.js";

const UPSTREAM_OWNER = "aeyakovenko";
const UPSTREAM_REPO = "percolator";
const FEATURE_BRANCH = "hypercolator-feature";

const FORK_POLL_INTERVAL_MS = 2000;
const FORK_POLL_MAX_ATTEMPTS = 15;

function headers(token: string): Record<string, string> {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "hypercolator-bot",
    "Content-Type": "application/json",
  };
}

async function checkPermissions(
  token: string,
  owner: string,
  repo: string
): Promise<void> {
  const res = await fetch(`https://api.github.com/repos/${owner}/${repo}`, {
    headers: headers(token),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(
      `Preflight check failed for ${owner}/${repo}: ${res.status} ${body}`
    );
  }
  log(`Preflight OK - can access ${owner}/${repo}`);
}

async function pollUntilForkReady(
  token: string,
  username: string
): Promise<void> {
  log("Polling until fork is ready...");
  for (let attempt = 1; attempt <= FORK_POLL_MAX_ATTEMPTS; attempt++) {
    await new Promise((r) => setTimeout(r, FORK_POLL_INTERVAL_MS));
    const res = await fetch(
      `https://api.github.com/repos/${username}/${UPSTREAM_REPO}`,
      { headers: headers(token) }
    );
    if (res.ok) {
      const data = (await res.json()) as { full_name: string };
      log(`Fork ready: ${data.full_name} (attempt ${attempt})`);
      return;
    }
    log(`Fork not ready yet (attempt ${attempt}/${FORK_POLL_MAX_ATTEMPTS})...`);
  }
  throw new Error(
    `Fork did not become ready after ${FORK_POLL_MAX_ATTEMPTS} attempts`
  );
}

export async function forkRepo(token: string, username: string): Promise<void> {
  log(`Forking ${UPSTREAM_OWNER}/${UPSTREAM_REPO} into ${username}...`);

  // Preflight: verify we can read the upstream repo
  await checkPermissions(token, UPSTREAM_OWNER, UPSTREAM_REPO);

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
        name: UPSTREAM_REPO,
        default_branch_only: false,
      }),
    }
  );

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Fork failed: ${res.status} ${body}`);
  }

  log(`Fork request accepted for github.com/${username}/${UPSTREAM_REPO}`);
  await pollUntilForkReady(token, username);
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

  // Resolve the actual default branch (could be master, main, or anything)
  const repoInfoRes = await fetch(`https://api.github.com/repos/${repoPath}`, {
    headers: headers(token),
  });

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
