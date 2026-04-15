# scripts/github - GitHub App Automation

Handles all GitHub operations for Hypercolator using a GitHub App
(App ID + PEM private key) instead of a Personal Access Token.

## Why GitHub App instead of PAT

GitHub Apps have a separate identity from your personal account.
They cannot trigger account-level rate limits or bans that affect PATs.
They use short-lived installation tokens (1 hour) that are auto-refreshed.

## Setup - Create the GitHub App

1. Go to https://github.com/settings/apps/new
2. Fill in:
   - GitHub App name: `hypercolator-bot` (or any name)
   - Homepage URL: any URL (e.g. `https://github.com`)
   - Uncheck "Active" under Webhook
3. Set Permissions (Repository permissions):
   - Contents: Read and Write
   - Issues: Read and Write
   - Pull requests: Read and Write
   - Metadata: Read (auto-selected, required)
4. Click "Create GitHub App"
5. On the App page, copy the **App ID** (a number, e.g. `1234567`)
6. Scroll to "Private keys" section, click "Generate a private key"
   - A `.pem` file downloads automatically
7. Go to "Install App" tab, click "Install" next to your account
8. After install, look at the browser URL - it ends with `/installations/XXXXXXXX`
   - Copy that number - that is your **Installation ID**

## Setup - Add Replit Secrets

In the Replit Secrets panel, add exactly these four secrets:

| Secret Name | Where to find it | Example format |
| --- | --- | --- |
| `GITHUB_APP_ID` | App settings page, top section | `1234567` |
| `GITHUB_APP_PRIVATE_KEY` | Contents of the downloaded `.pem` file | `-----BEGIN RSA PRIVATE KEY-----\nMIIE...` |
| `GITHUB_APP_INSTALLATION_ID` | URL after installing App: `/installations/XXXXXXXX` | `12345678` |
| `GITHUB_USERNAME` | Your GitHub username | `codes-son` |

### Formatting GITHUB_APP_PRIVATE_KEY

Open the downloaded `.pem` file in a text editor.
Copy the entire contents including the header and footer lines:

```
-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEA...
...many lines...
-----END RSA PRIVATE KEY-----
```

When pasting into Replit Secrets, you can paste it as-is with real newlines,
or replace each newline with a literal `\n` character sequence.
Both formats are supported - the auth module normalizes them automatically.

## Running the bootstrap

```
pnpm --filter @workspace/github run run
```

This will:
1. Authenticate and get an installation token
2. Fork `aeyakovenko/percolator` to your account
3. Create branch `hypercolator-feature` on your fork
4. Open an architecture feedback issue on Toly's repo

All steps are idempotent - safe to run multiple times.

## Module reference

- `auth.ts` - JWT generation and installation token exchange
- `fork.ts` - Fork repo and create branches (uses dynamic default branch detection)
- `issues.ts` - Open issues on GitHub repos
- `run.ts` - Main orchestration script
