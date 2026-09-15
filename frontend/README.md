# Frontend — Stellar Transaction Auditor

React + TypeScript + Vite frontend for the Stellar Transaction Auditor.

## Features

- **Dashboard** — Overview stats and recent entries
- **Audit Log** — Paginated table with entry detail modal
- **Record** — Submit new audit entries via the API
- **Verify** — Hash-chain integrity verification
- **Admin** — Manage auditor roles, pause/unpause recording
- **Wallet** — Freighter wallet integration (connect, balance, send XLM)
- **Dark mode** — Theme toggle persisted to localStorage
- **Mobile responsive** — Works on all screen sizes

## Development

```bash
npm install
npm run dev    # Dev server at :5173 (proxies /api to backend at :3000)
npm run build  # Production build to dist/
```

## API Proxy

The Vite dev server proxies `/api` and `/health` to `http://localhost:3000`.
Make sure the backend is running before starting the frontend.

## Tech Stack

- React 18
- TypeScript 5.6
- Vite 5.4
- @stellar/freighter-api (wallet)
- stellar-sdk (Stellar operations)
