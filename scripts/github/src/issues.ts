/**
 * GitHub Issues utility.
 *
 * Opens a structured issue on the upstream Percolator repo
 * with architecture feedback from the Hypercolator team.
 */

import { log, warn } from "./logger.js";

const UPSTREAM_OWNER = "aeyakovenko";
const UPSTREAM_REPO = "percolator";

const ISSUE_TITLE = "Architecture feedback request \u2014 Hypercolator extension";

const ISSUE_BODY = `Hi Toly and team,

We are building Hypercolator, a permissionless perpetual futures DEX on Solana built on top of the Percolator risk engine.

Our goals:
- Wrap the Percolator risk engine in Anchor instructions deployable on-chain
- Enable permissionless market creation for any SPL token (including pump.fun tokens)
- TWAP-based price discovery from on-chain AMM pools (no external oracle required)
- Self-funded insurance via per-trade fee accumulation (0.08% per trade)
- Tier-based leverage limits (Tier A 50x, Tier B 20x, Tier C 5x for unknown tokens)

We have a few architecture questions and would appreciate feedback:

1. The risk engine is designed as a pure in-process library. Are there known blockers
   to using its core types (RiskEngine, Account, InsuranceFund) as Anchor account data?

2. The ADL system assumes a single shared ADL priority index. In a multi-market setup
   with independent InsuranceFund accounts per market, does the ADL invariant still hold?

3. For the TWAP integration - the engine accepts a wrapper-supplied oracle price.
   Is there guidance on acceptable price staleness bounds in the spec?

We plan to contribute documentation improvements and modular extension examples
back to this repo once our research phase is complete.

Repository: https://github.com/hypercolator/percolator (WIP - work in progress)

Thank you for the excellent work on this codebase.
`;

function headers(token: string): Record<string, string> {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
    "User-Agent": "hypercolator-bot",
    "Content-Type": "application/json",
  };
}

export async function openArchitectureIssue(token: string): Promise<number> {
  log(`Opening issue on ${UPSTREAM_OWNER}/${UPSTREAM_REPO}...`);

  // Search for existing issue by keyword to stay idempotent regardless of exact title format
  const searchKeyword = "Architecture+feedback+request+Hypercolator";
  const searchRes = await fetch(
    `https://api.github.com/search/issues?q=repo:${UPSTREAM_OWNER}/${UPSTREAM_REPO}+is:issue+${searchKeyword}`,
    { headers: headers(token) }
  );

  if (searchRes.ok) {
    const searchData = (await searchRes.json()) as {
      total_count: number;
      items: Array<{ number: number; title: string }>;
    };
    if (searchData.total_count > 0) {
      const existing = searchData.items[0];
      if (existing) {
        warn(
          `Issue already exists: #${existing.number} "${existing.title}" - skipping`
        );
        return existing.number;
      }
    }
  }

  const res = await fetch(
    `https://api.github.com/repos/${UPSTREAM_OWNER}/${UPSTREAM_REPO}/issues`,
    {
      method: "POST",
      headers: headers(token),
      body: JSON.stringify({
        title: ISSUE_TITLE,
        body: ISSUE_BODY,
        // No labels - upstream repo may not have these labels and GitHub
        // returns 422 if a label does not exist, causing unnecessary failure
      }),
    }
  );

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Issue creation failed: ${res.status} ${body}`);
  }

  const data = (await res.json()) as { number: number; html_url: string };
  log(`Issue #${data.number} opened: ${data.html_url}`);
  return data.number;
}
