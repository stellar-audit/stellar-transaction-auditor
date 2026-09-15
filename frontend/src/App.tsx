import { useCallback, useEffect, useState } from "react";
import { api, AuditEntry, Category, PaginatedEntries, VerifyResult } from "./api";
import {
  connectWallet,
  disconnectWallet,
  getActiveAddress,
  getFreighterNetwork,
} from "./wallet";
import { getXlmBalance, NETWORK_NAME } from "./stellar";
import "./styles.css";

type Tab = "dashboard" | "entries" | "record" | "verify" | "admin" | "wallet";

type Toast = { kind: "success" | "error" | "info"; msg: string } | null;

const CATEGORIES: { value: Category; label: string }[] = [
  { value: "payment", label: "Payment" },
  { value: "contract_call", label: "Contract Call" },
  { value: "compliance", label: "Compliance" },
  { value: "security", label: "Security" },
  { value: "other", label: "Other" },
];

const PAGE_SIZE = 10;

export default function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  const [dark, setDark] = useState(() => localStorage.getItem("dark") === "1");
  const [toast, setToast] = useState<Toast>(null);
  const [walletAddr, setWalletAddr] = useState<string | null>(null);
  const [walletNet, setWalletNet] = useState<string | null>(null);
  const [balance, setBalance] = useState<string | null>(null);
  const [entryCount, setEntryCount] = useState(0);
  const [paused, setPaused] = useState(false);
  const [apiVersion, setApiVersion] = useState("");

  // Apply dark mode class
  useEffect(() => {
    document.documentElement.className = dark ? "dark" : "";
    localStorage.setItem("dark", dark ? "1" : "0");
  }, [dark]);

  // Auto-dismiss toast
  useEffect(() => {
    if (toast) {
      const t = setTimeout(() => setToast(null), 4000);
      return () => clearTimeout(t);
    }
  }, [toast]);

  // Reconnect wallet on load
  useEffect(() => {
    (async () => {
      const a = await getActiveAddress();
      if (a) {
        setWalletAddr(a);
        const net = await getFreighterNetwork();
        setWalletNet(net?.network ?? null);
      }
    })();
  }, []);

  // Fetch summary data
  const refreshSummary = useCallback(async () => {
    try {
      const [count, st, hp] = await Promise.all([
        api.entryCount(),
        api.status(),
        api.isPaused(),
      ]);
      setEntryCount(count.count);
      setApiVersion(st.version);
      setPaused(hp.paused);
    } catch {
      // backend may not be running
    }
  }, []);

  useEffect(() => {
    refreshSummary();
    const interval = setInterval(refreshSummary, 30000);
    return () => clearInterval(interval);
  }, [refreshSummary]);

  const showToast = (kind: "success" | "error" | "info", msg: string) =>
    setToast({ kind, msg });

  const onConnect = async () => {
    const res = await connectWallet();
    if (res.error) {
      showToast("error", res.error);
      return;
    }
    setWalletAddr(res.address!);
    const net = await getFreighterNetwork();
    setWalletNet(net?.network ?? null);
    try {
      setBalance(await getXlmBalance(res.address!));
    } catch {}
    showToast("success", "Wallet connected");
  };

  const onDisconnect = async () => {
    await disconnectWallet();
    setWalletAddr(null);
    setWalletNet(null);
    setBalance(null);
    showToast("info", "Wallet disconnected");
  };

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <div className="header-left">
          <h1 className="logo">🔐 Stellar Audit</h1>
          <span className="badge">v{apiVersion || "—"}</span>
          {paused && <span className="badge warn">⏸ Paused</span>}
        </div>
        <div className="header-right">
          <span className="entry-count">{entryCount} entries</span>
          <button className="icon-btn" onClick={() => setDark(!dark)} title="Toggle theme">
            {dark ? "☀️" : "🌙"}
          </button>
          {walletAddr ? (
            <div className="wallet-pill">
              <code>{walletAddr.slice(0, 6)}…{walletAddr.slice(-4)}</code>
              <button onClick={onDisconnect}>Disconnect</button>
            </div>
          ) : (
            <button className="primary" onClick={onConnect}>Connect Freighter</button>
          )}
        </div>
      </header>

      {/* Nav tabs */}
      <nav className="tabs">
        {(["dashboard", "entries", "record", "verify", "admin", "wallet"] as Tab[]).map((t) => (
          <button
            key={t}
            className={tab === t ? "tab active" : "tab"}
            onClick={() => setTab(t)}
          >
            {t === "dashboard" && "📊 Dashboard"}
            {t === "entries" && "📋 Audit Log"}
            {t === "record" && "✏️ Record"}
            {t === "verify" && "✅ Verify"}
            {t === "admin" && "⚙️ Admin"}
            {t === "wallet" && "👛 Wallet"}
          </button>
        ))}
      </nav>

      {/* Main content */}
      <main className="main">
        {tab === "dashboard" && (
          <Dashboard entryCount={entryCount} paused={paused} onRefresh={refreshSummary} />
        )}
        {tab === "entries" && <EntriesView showToast={showToast} />}
        {tab === "record" && (
          <RecordForm auditor={walletAddr} showToast={showToast} onRecorded={refreshSummary} />
        )}
        {tab === "verify" && <VerifyView showToast={showToast} entryCount={entryCount} />}
        {tab === "admin" && <AdminPanel showToast={showToast} onRefresh={refreshSummary} />}
        {tab === "wallet" && (
          <WalletView
            addr={walletAddr}
            net={walletNet}
            balance={balance}
            onConnect={onConnect}
            onRefreshBalance={async () => {
              if (walletAddr) {
                try {
                  setBalance(await getXlmBalance(walletAddr));
                } catch {}
              }
            }}
          />
        )}
      </main>

      {/* Toast */}
      {toast && (
        <div className={`toast ${toast.kind}`}>
          {toast.kind === "success" ? "✅" : toast.kind === "error" ? "❌" : "ℹ️"} {toast.msg}
        </div>
      )}

      <footer className="footer">
        <p>Stellar Transaction Auditor · Testnet · MIT License</p>
      </footer>
    </div>
  );
}

// --- Dashboard ---
function Dashboard({
  entryCount,
  paused,
  onRefresh,
}: {
  entryCount: number;
  paused: boolean;
  onRefresh: () => void;
}) {
  const [status, setStatus] = useState<awaited<ReturnType<typeof api.status>> | null>(null);
  const [recent, setRecent] = useState<AuditEntry[]>([]);

  useEffect(() => {
    (async () => {
      try {
        const [st, data] = await Promise.all([api.status(), api.listEntries(0, 5)]);
        setStatus(st);
        setRecent(data.entries);
      } catch {}
    })();
  }, [entryCount]);

  return (
    <div className="dashboard">
      <div className="stats-grid">
        <div className="stat-card">
          <span className="stat-label">Total Entries</span>
          <span className="stat-value">{entryCount}</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Status</span>
          <span className="stat-value">{paused ? "⏸ Paused" : "✅ Active"}</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Network</span>
          <span className="stat-value">{status?.network || "—"}</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Uptime</span>
          <span className="stat-value">
            {status ? `${Math.floor(status.uptime_seconds / 60)}m` : "—"}
          </span>
        </div>
      </div>

      <div className="card">
        <h2>Recent Entries</h2>
        {recent.length === 0 ? (
          <p className="muted">No entries yet.</p>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>#</th>
                <th>Category</th>
                <th>Subject</th>
                <th>Hash</th>
                <th>Memo</th>
              </tr>
            </thead>
            <tbody>
              {recent.map((e) => (
                <tr key={e.id}>
                  <td>{e.id}</td>
                  <td>
                    <span className={`cat cat-${e.category}`}>{e.category}</span>
                  </td>
                  <td>
                    <code className="addr">{e.subject.slice(0, 10)}…</code>
                  </td>
                  <td>
                    <code className="hash">{e.entry_hash.slice(0, 16)}…</code>
                  </td>
                  <td className="memo-cell">{e.memo}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <button onClick={onRefresh}>🔄 Refresh</button>
    </div>
  );
}

// --- Entries View with pagination ---
function EntriesView({ showToast }: { showToast: (k: "success" | "error" | "info", m: string) => void }) {
  const [data, setData] = useState<PaginatedEntries | null>(null);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<AuditEntry | null>(null);

  const load = useCallback(async (p: number) => {
    setLoading(true);
    try {
      const d = await api.listEntries(p * PAGE_SIZE, PAGE_SIZE);
      setData(d);
    } catch (e: any) {
      showToast("error", `Failed to load entries: ${e.message}`);
    } finally {
      setLoading(false);
    }
  }, [showToast]);

  useEffect(() => {
    load(page);
  }, [page, load]);

  const totalPages = data ? Math.ceil(data.total / PAGE_SIZE) : 0;

  return (
    <div>
      <div className="card">
        <h2>Audit Log</h2>
        {loading ? (
          <p className="muted">Loading…</p>
        ) : !data || data.entries.length === 0 ? (
          <p className="muted">No entries recorded.</p>
        ) : (
          <>
            <div className="table-scroll">
              <table className="table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Auditor</th>
                    <th>Subject</th>
                    <th>Category</th>
                    <th>Timestamp</th>
                    <th>Entry Hash</th>
                    <th>Memo</th>
                  </tr>
                </thead>
                <tbody>
                  {data.entries.map((e) => (
                    <tr key={e.id} onClick={() => setSelected(e)} className="clickable">
                      <td>{e.id}</td>
                      <td>
                        <code className="addr">{e.auditor.slice(0, 10)}…</code>
                      </td>
                      <td>
                        <code className="addr">{e.subject.slice(0, 10)}…</code>
                      </td>
                      <td>
                        <span className={`cat cat-${e.category}`}>{e.category}</span>
                      </td>
                      <td className="ts">{new Date(e.timestamp * 1000).toLocaleString()}</td>
                      <td>
                        <code className="hash">{e.entry_hash.slice(0, 16)}…</code>
                      </td>
                      <td className="memo-cell">{e.memo}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="pagination">
              <button disabled={page === 0} onClick={() => setPage(page - 1)}>
                ← Prev
              </button>
              <span>
                Page {page + 1} of {totalPages || 1} ({data?.total} total)
              </span>
              <button
                disabled={page + 1 >= totalPages}
                onClick={() => setPage(page + 1)}
              >
                Next →
              </button>
            </div>
          </>
        )}
      </div>

      {selected && (
        <div className="modal-overlay" onClick={() => setSelected(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>Entry #{selected.id}</h2>
            <div className="detail-grid">
              <DetailRow label="Auditor" value={selected.auditor} mono />
              <DetailRow label="Subject" value={selected.subject} mono />
              <DetailRow label="Category" value={selected.category} />
              <DetailRow label="Data Hash" value={selected.data_hash} mono />
              <DetailRow label="Prev Hash" value={selected.prev_hash} mono />
              <DetailRow label="Entry Hash" value={selected.entry_hash} mono />
              <DetailRow label="Timestamp" value={new Date(selected.timestamp * 1000).toISOString()} />
              <DetailRow label="Ledger" value={String(selected.ledger)} />
              <DetailRow label="Expires At" value={new Date(selected.expires_at * 1000).toISOString()} />
              <DetailRow label="Memo" value={selected.memo} />
            </div>
            <button onClick={() => setSelected(null)}>Close</button>
          </div>
        </div>
      )}
    </div>
  );
}

function DetailRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="detail-row">
      <span className="detail-label">{label}</span>
      <span className={mono ? "mono" : ""}>{value}</span>
    </div>
  );
}

// --- Record Form ---
function RecordForm({
  auditor,
  showToast,
  onRecorded,
}: {
  auditor: string | null;
  showToast: (k: "success" | "error" | "info", m: string) => void;
  onRecorded: () => void;
}) {
  const [subject, setSubject] = useState("");
  const [category, setCategory] = useState<Category>("payment");
  const [dataHash, setDataHash] = useState("");
  const [memo, setMemo] = useState("");
  const [expiryHours, setExpiryHours] = useState("24");
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!auditor) {
      showToast("error", "Connect your wallet first (Wallet tab)");
      return;
    }
    if (!subject || !dataHash) {
      showToast("error", "Subject and data hash are required");
      return;
    }
    setSubmitting(true);
    try {
      const expiresAt = Math.floor(Date.now() / 1000) + parseInt(expiryHours) * 3600;
      const res = await api.recordEntry({
        auditor,
        subject,
        category,
        data_hash: dataHash,
        memo: memo || "(no memo)",
        expires_at: expiresAt,
      });
      showToast("success", res.message);
      setSubject("");
      setDataHash("");
      setMemo("");
      onRecorded();
    } catch (e: any) {
      showToast("error", e.message);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="card">
      <h2>Record Audit Entry</h2>
      <p className="muted">
        Auditor: {auditor ? <code>{auditor.slice(0, 12)}…</code> : "⚠️ Connect wallet first"}
      </p>
      <form onSubmit={handleSubmit} className="form">
        <label>
          Subject Address
          <input
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
            placeholder="G… (account being audited)"
          />
        </label>
        <label>
          Category
          <select value={category} onChange={(e) => setCategory(e.target.value as Category)}>
            {CATEGORIES.map((c) => (
              <option key={c.value} value={c.value}>
                {c.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          Data Hash (SHA-256, hex)
          <input
            value={dataHash}
            onChange={(e) => setDataHash(e.target.value)}
            placeholder="e.g. 9f98196d…"
          />
        </label>
        <label>
          Memo (max 512 chars)
          <textarea
            value={memo}
            onChange={(e) => setMemo(e.target.value)}
            placeholder="Optional note about this audit entry"
            maxLength={512}
            rows={3}
          />
        </label>
        <label>
          Expiry (hours from now)
          <input
            type="number"
            value={expiryHours}
            onChange={(e) => setExpiryHours(e.target.value)}
            min="1"
          />
        </label>
        <button type="submit" className="primary" disabled={submitting || !auditor}>
          {submitting ? "Recording…" : "Record Entry"}
        </button>
      </form>
    </div>
  );
}

// --- Verify View ---
function VerifyView({
  showToast,
  entryCount,
}: {
  showToast: (k: "success" | "error" | "info", m: string) => void;
  entryCount: number;
}) {
  const [from, setFrom] = useState("0");
  const [to, setTo] = useState(String(Math.max(0, entryCount - 1)));
  const [result, setResult] = useState<VerifyResult | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (entryCount > 0) setTo(String(entryCount - 1));
  }, [entryCount]);

  const handleVerify = async () => {
    const f = parseInt(from);
    const t = parseInt(to);
    if (isNaN(f) || isNaN(t) || f > t) {
      showToast("error", "Invalid range: from must be <= to");
      return;
    }
    setLoading(true);
    try {
      const r = await api.verifyChain(f, t);
      setResult(r);
      showToast(r.verified ? "success" : "error", r.verified ? "Chain verified!" : "Chain BROKEN — tampering detected!");
    } catch (e: any) {
      showToast("error", e.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="card">
      <h2>Chain Verification</h2>
      <p className="muted">
        Verify the integrity of the hash-chained audit log. Any tampered entry
        will cause verification to fail.
      </p>
      <div className="verify-form">
        <label>
          From entry #
          <input type="number" value={from} onChange={(e) => setFrom(e.target.value)} min="0" />
        </label>
        <label>
          To entry #
          <input type="number" value={to} onChange={(e) => setTo(e.target.value)} min="0" />
        </label>
        <button className="primary" onClick={handleVerify} disabled={loading}>
          {loading ? "Verifying…" : "Verify Chain"}
        </button>
      </div>
      {result && (
        <div className={`verify-result ${result.verified ? "ok" : "fail"}`}>
          <h3>{result.verified ? "✅ Chain Integrity Verified" : "❌ Chain Broken"}</h3>
          <p>Range: entries {result.from} to {result.to}</p>
          <p>Entries checked: {result.entries_checked}</p>
          <p>
            {result.verified
              ? "All hashes match and chain links are intact. The audit log has not been tampered with."
              : "At least one entry's hash or chain link is invalid. The log may have been tampered with."}
          </p>
        </div>
      )}
    </div>
  );
}

// --- Admin Panel ---
function AdminPanel({
  showToast,
  onRefresh,
}: {
  showToast: (k: "success" | "error" | "info", m: string) => void;
  onRefresh: () => void;
}) {
  const [adminAddr, setAdminAddr] = useState("");
  const [checkAddr, setCheckAddr] = useState("");
  const [auditorResult, setAuditorResult] = useState<boolean | null>(null);
  const [grantAddr, setGrantAddr] = useState("");
  const [revokeAddr, setRevokeAddr] = useState("");
  const [isPaused, setIsPaused] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const a = await api.getAdmin();
        setAdminAddr(a.admin);
        const p = await api.isPaused();
        setIsPaused(p.paused);
      } catch {}
    })();
  }, []);

  const checkAuditor = async () => {
    if (!checkAddr) return;
    try {
      const r = await api.checkAuditor(checkAddr);
      setAuditorResult(r.is_auditor);
    } catch (e: any) {
      showToast("error", e.message);
    }
  };

  const handleGrant = async () => {
    if (!grantAddr || !adminAddr) return;
    try {
      const r = await api.grantAuditor(adminAddr, grantAddr);
      showToast("success", r.message);
      setGrantAddr("");
    } catch (e: any) {
      showToast("error", e.message);
    }
  };

  const handleRevoke = async () => {
    if (!revokeAddr || !adminAddr) return;
    try {
      const r = await api.revokeAuditor(adminAddr, revokeAddr);
      showToast("success", r.message);
      setRevokeAddr("");
    } catch (e: any) {
      showToast("error", e.message);
    }
  };

  const togglePause = async () => {
    try {
      if (isPaused) {
        await api.unpause();
        setIsPaused(false);
        showToast("success", "Recording resumed");
      } else {
        await api.pause();
        setIsPaused(true);
        showToast("info", "Recording paused");
      }
      onRefresh();
    } catch (e: any) {
      showToast("error", e.message);
    }
  };

  return (
    <div className="admin-panel">
      <div className="card">
        <h2>Admin Panel</h2>
        <div className="detail-row">
          <span className="detail-label">Admin Address:</span>
          <code className="mono">{adminAddr || "—"}</code>
        </div>
        <div className="detail-row">
          <span className="detail-label">Recording Status:</span>
          <span className={isPaused ? "warn" : "ok"}>
            {isPaused ? "⏸ Paused" : "✅ Active"}
          </span>
        </div>
        <button className={isPaused ? "primary" : ""} onClick={togglePause}>
          {isPaused ? "▶️ Resume Recording" : "⏸ Pause Recording"}
        </button>
      </div>

      <div className="card">
        <h3>Check Auditor Role</h3>
        <div className="inline-form">
          <input
            value={checkAddr}
            onChange={(e) => setCheckAddr(e.target.value)}
            placeholder="G… address to check"
          />
          <button onClick={checkAuditor}>Check</button>
        </div>
        {auditorResult !== null && (
          <p className={auditorResult ? "ok" : "warn"}>
            {auditorResult ? "✅ This address IS an auditor" : "❌ This address is NOT an auditor"}
          </p>
        )}
      </div>

      <div className="card">
        <h3>Grant Auditor Role</h3>
        <div className="inline-form">
          <input
            value={grantAddr}
            onChange={(e) => setGrantAddr(e.target.value)}
            placeholder="G… address to grant"
          />
          <button className="primary" onClick={handleGrant}>Grant</button>
        </div>
      </div>

      <div className="card">
        <h3>Revoke Auditor Role</h3>
        <div className="inline-form">
          <input
            value={revokeAddr}
            onChange={(e) => setRevokeAddr(e.target.value)}
            placeholder="G… address to revoke"
          />
          <button onClick={handleRevoke}>Revoke</button>
        </div>
      </div>
    </div>
  );
}

// --- Wallet View ---
function WalletView({
  addr,
  net,
  balance,
  onConnect,
  onRefreshBalance,
}: {
  addr: string | null;
  net: string | null;
  balance: string | null;
  onConnect: () => void;
  onRefreshBalance: () => void;
}) {
  return (
    <div className="card">
      <h2>Freighter Wallet</h2>
      {!addr ? (
        <div>
          <p className="muted">Connect your Freighter wallet to interact with the audit log.</p>
          <button className="primary" onClick={onConnect}>Connect Freighter</button>
        </div>
      ) : (
        <div>
          <div className="detail-row">
            <span className="detail-label">Address:</span>
            <code className="mono">{addr}</code>
          </div>
          <div className="detail-row">
            <span className="detail-label">Network:</span>
            <span className={net === NETWORK_NAME ? "ok" : "warn"}>
              {net ?? "unknown"}
              {net !== NETWORK_NAME ? " (switch to Testnet!)" : ""}
            </span>
          </div>
          <div className="detail-row">
            <span className="detail-label">XLM Balance:</span>
            <strong>{balance ?? "—"}</strong>
          </div>
          <button onClick={onRefreshBalance}>🔄 Refresh Balance</button>
        </div>
      )}
    </div>
  );
}
