# Companion

AI-powered work companion that aggregates content from Slack and Atlassian, summarizes it with Google Gemini, and presents actionable daily and weekly digests.

## Features

### Data Sources

- **Slack** - Sync messages from selected channels with full thread support
- **Jira** - Sync issues and tickets from your projects
- **Confluence** - Sync pages and documentation

### AI-Powered Intelligence

Companion uses Google Gemini to automatically:

- **Summarize** conversations and documents into concise overviews
- **Categorize** content by department (Engineering, Product, Sales, Marketing, Research)
- **Score importance** to surface what matters most
- **Extract entities** including people, projects, and key topics
- **Group discussions** that span multiple channels into unified topics
- **Identify action items** from conversations

### Views

- **Daily Digest** - Browse summarized content by day with category filters
- **Weekly Summary** - Timeline view of the entire week's activity
- **Settings** - Configure integrations, API keys, sync schedule, and appearance

### Privacy & Security

- All content is **encrypted at rest** using AES-GCM
- Credentials stored securely in your **OS keychain** (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Data stays **local** on your machine in a SQLite database
- No data is sent anywhere except the configured AI provider

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/companion.git
cd companion

# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev
```

### Build for Production

```bash
pnpm tauri build
```

## Configuration

### Gemini API Key

1. Get your API key from [Google AI Studio](https://aistudio.google.com/app/apikey)
2. Open Companion → Settings → API Keys
3. Enter your Gemini API key and save

### Slack Integration

1. Go to [api.slack.com/apps](https://api.slack.com/apps) → Create New App → From scratch
2. Name it (e.g., "Companion") and select your workspace
3. Navigate to **OAuth & Permissions** and add these **User Token Scopes**:
   - `channels:history`, `channels:read`
   - `groups:history`, `groups:read`
   - `im:history`, `im:read`
   - `mpim:history`, `mpim:read`
   - `users:read`
4. Click **Install to Workspace** and authorize
5. Copy the **User OAuth Token** (starts with `xoxp-`)
6. Open Companion → Settings → Sources → Connect Slack
7. Paste your token and select which channels to sync

### Sync Settings

Configure how often Companion syncs data:
- Every 5, 15, 30, or 60 minutes
- Enable/disable individual sources

## Tech Stack

### Frontend
- [React 19](https://react.dev/) with TypeScript
- [Tailwind CSS](https://tailwindcss.com/) for styling
- [TanStack Query](https://tanstack.com/query) for data fetching
- [Zustand](https://zustand-demo.pmnd.rs/) for state management
- [Lucide React](https://lucide.dev/) for icons

### Backend
- [Tauri 2](https://tauri.app/) - Rust-based desktop framework
- [SQLx](https://github.com/launchbadge/sqlx) with SQLite
- [Tokio](https://tokio.rs/) async runtime
- [Reqwest](https://github.com/seanmonstar/reqwest) HTTP client

### Security
- [aes-gcm](https://github.com/RustCrypto/AEADs) for encryption
- [keyring](https://github.com/hwchen/keyring-rs) for OS credential storage

## Development

```bash
# Run frontend only
pnpm dev

# Run tests
pnpm test

# Run tests in watch mode
pnpm test:watch

# Lint
pnpm lint
```

## License

MIT
