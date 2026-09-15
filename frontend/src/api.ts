// API client for the Stellar Transaction Auditor backend.
// All calls go through the Vite dev-server proxy (see vite.config.ts).

const BASE = "/api/v1";

export type Category = "payment" | "contract_call" | "compliance" | "security" | "other";

export interface AuditEntry {
  id: number;
  auditor: string;
  subject: string;
  category: Category;
  data_hash: string;
  prev_hash: string;
  entry_hash: string;
  timestamp: number;
  ledger: number;
  expires_at: number;
  memo: string;
}

export interface PaginatedEntries {
  entries: AuditEntry[];
  count: number;
  start: number;
  limit: number;
  total: number;
}

export interface VerifyResult {
  from: number;
  to: number;
  verified: boolean;
  entries_checked: number;
}

export interface HealthResponse {
  status: string;
  version: string;
  uptime_seconds: number;
}

export interface StatusResponse {
  status: string;
  version: string;
  uptime_seconds: number;
  network: string;
  contract_id: string;
  soroban_rpc_url: string;
  horizon_url: string;
  entry_count: number;
}

export interface RecordRequest {
  auditor: string;
  subject: string;
  category: Category;
  data_hash: string;
  memo: string;
  expires_at: number;
}

export interface OperationResult {
  success: boolean;
  message: string;
}

async function jsonFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }));
    throw new Error(err.message || `HTTP ${res.status}`);
  }
  return res.json();
}

export const api = {
  health: () => jsonFetch<HealthResponse>("/health"),
  status: () => jsonFetch<StatusResponse>(`${BASE}/status`),
  listEntries: (start = 0, limit = 20) =>
    jsonFetch<PaginatedEntries>(`${BASE}/entries?start=${start}&limit=${limit}`),
  getEntry: (id: number) => jsonFetch<AuditEntry>(`${BASE}/entries/${id}`),
  entryCount: () => jsonFetch<{ count: number }>(`${BASE}/entries/count`),
  recordEntry: (req: RecordRequest) =>
    jsonFetch<OperationResult>(`${BASE}/entries`, {
      method: "POST",
      body: JSON.stringify(req),
    }),
  verifyChain: (from: number, to: number) =>
    jsonFetch<VerifyResult>(`${BASE}/verify?from=${from}&to=${to}`),
  getAdmin: () => jsonFetch<{ admin: string }>(`${BASE}/admin`),
  checkAuditor: (addr: string) =>
    jsonFetch<{ address: string; is_auditor: boolean }>(`${BASE}/auditors/${addr}`),
  grantAuditor: (admin: string, address: string) =>
    jsonFetch<OperationResult>(`${BASE}/admin/grant`, {
      method: "POST",
      body: JSON.stringify({ admin, address }),
    }),
  revokeAuditor: (admin: string, address: string) =>
    jsonFetch<OperationResult>(`${BASE}/admin/revoke`, {
      method: "POST",
      body: JSON.stringify({ admin, address }),
    }),
  pause: () => jsonFetch<OperationResult>(`${BASE}/admin/pause`, { method: "POST" }),
  unpause: () => jsonFetch<OperationResult>(`${BASE}/admin/unpause`, { method: "POST" }),
  isPaused: () => jsonFetch<{ paused: boolean }>(`${BASE}/admin/paused`),
};
