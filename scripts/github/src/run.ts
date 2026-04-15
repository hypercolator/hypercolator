/**
 * Main runner for GitHub App bootstrap operations.
 *
 * Executes in order:
 *   1. Authenticate via GitHub App (App ID + PEM -> installation token)
 *   2. Fork aeyakovenko/percolator to user account
 *   3. Create hypercolator-feature branch on the fork
 *   4. Open architecture feedback issue on the upstream repo
 *
 * Required secrets (set in Replit Secrets panel):
 *   GITHUB_APP_ID
 *   GITHUB_APP_PRIVATE_KEY
 *   GITHUB_APP_INSTALLATION_ID
 *   GITHUB_USERNAME
 */

import { getInstallationToken } from "./auth.js";
import { forkRepo, createFeatureBranch } from "./fork.js";
import { openArchitectureIssue } from "./issues.js";
import { log, error } from "./logger.js";

async function main(): Promise<void> {
  const username = process.env.GITHUB_USERNAME;
  if (!username) {
    throw new Error("Missing required secret: GITHUB_USERNAME");
  }

  log("=== Hypercolator GitHub Bootstrap ===");
  log(`Target user: ${username}`);

  // Step 1 - authenticate
  log("Step 1: Authenticating via GitHub App...");
  const token = await getInstallationToken();

  // Step 2 - fork the repo
  log("Step 2: Forking aeyakovenko/percolator...");
  await forkRepo(token, username);

  // Step 3 - create feature branch
  log("Step 3: Creating hypercolator-feature branch...");
  await createFeatureBranch(token, username);

  // Step 4 - open issue on upstream
  log("Step 4: Opening architecture feedback issue on upstream repo...");
  const issueNumber = await openArchitectureIssue(token);

  log("=== Bootstrap Complete ===");
  log(`Fork: https://github.com/${username}/percolator`);
  log(
    `Branch: https://github.com/${username}/percolator/tree/hypercolator-feature`
  );
  log(
    `Issue: https://github.com/aeyakovenko/percolator/issues/${issueNumber}`
  );
}

main().catch((err: Error) => {
  error(err.message);
  process.exit(1);
});
