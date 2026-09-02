import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";

export interface CommandBridge {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

interface MemberRow {
  id: string;
  name: string;
  role: string;
  masked_pan: string;
}

interface FriendRow {
  id: string;
  owner_member_id: string;
  label: string;
  masked_pan: string;
  share_basis_points: number;
}

interface DashboardData {
  total_planned_paise: number;
  submitted_session_count: number;
  member_count: number;
  friend_count: number;
  profit_paise: number;
}

interface RankedIpo {
  typed_name: string;
  ranking: number;
  score: number;
  recommended_account_count: number;
  recommended_allocation_ratio_bp: number;
  skip: boolean;
  reason: string;
  missing_public_data: string[];
}

interface CheckResponse {
  session_id: string;
  algorithm_version: string;
  label: string;
  explanation: string;
  ipos: RankedIpo[];
}

interface IpoDraft {
  id: string;
  name: string;
  amountRupees: string;
  registrarId: string;
  expectedAllotmentDate: string;
  included: boolean;
  lots: string;
  metadataSnapshot: IpoMetadataSnapshot | null;
}

interface IpoMetadataSnapshot {
  metadata_source: "UPSTOX_IPO_API";
  source_ipo_id: string;
  source_status: "OPEN";
  source_name: string;
  source_symbol: string;
  source_isin: string | null;
  fetched_at: string;
  revalidated_at: string | null;
  minimum_price_paise: number | null;
  maximum_price_paise: number | null;
  cut_off_price_paise: number | null;
  planning_price_paise: number | null;
  price_basis: "CUT_OFF" | "UPPER_BAND_ESTIMATE" | "TBA";
  lot_size: number | null;
  minimum_quantity: number | null;
  minimum_lots: number | null;
  lots: number;
  quantity: number;
  amount_per_account_paise: number;
  total_capital_paise: number;
  bidding_start_date: string | null;
  bidding_end_date: string | null;
  allotment_date: string | null;
  listing_date: string | null;
  registrar_name: string | null;
  registrar_short_name: string | null;
  registrar_website: string | null;
}

interface IpoCatalogItem {
  source: string;
  source_ipo_id: string;
  isin: string | null;
  issue_size_crore: string | null;
  industry: string | null;
  symbol: string;
  name: string;
  issue_type: "REGULAR" | "SME";
  status: "OPEN" | "UPCOMING" | "CLOSED" | "LISTED";
  minimum_price_paise: number | null;
  maximum_price_paise: number | null;
  cut_off_price_paise: number | null;
  planning_price_paise: number | null;
  price_basis: "CUT_OFF" | "UPPER_BAND_ESTIMATE" | "TBA";
  lot_size: number | null;
  minimum_quantity: number | null;
  minimum_lots: number | null;
  cost_per_lot_paise: number | null;
  minimum_application_amount_paise: number | null;
  bidding_start_date: string | null;
  bidding_end_date: string | null;
  allotment_date: string | null;
  listing_date: string | null;
  pre_apply_start_date: string | null;
  allotment_start_date: string | null;
  refund_initiation_date: string | null;
  mandate_end_date: string | null;
  daily_start_time: string | null;
  daily_end_time: string | null;
  face_value_paise: number | null;
  tick_size_paise: number | null;
  listing_price_paise: number | null;
  rhp_url: string | null;
  drhp_url: string | null;
  registrar_name: string | null;
  registrar_short_name: string | null;
  registrar_email: string | null;
  registrar_contact_name: string | null;
  registrar_contact_number: string | null;
  registrar_website: string | null;
  registrar_mapping_state: "CONFIRMED_ALIAS" | "UNKNOWN";
  listing_exchange: string | null;
  total_subscription: string | null;
  fetched_at: string;
  stale: boolean;
  safe_message: string | null;
}

interface IpoCatalogue {
  status: "OPEN" | "UPCOMING";
  items: IpoCatalogItem[];
  stale: boolean;
  fetched_at: string;
  safe_message: string | null;
}

interface HistoricalApplicationResponse {
  session_id: string;
  application_id: string;
  allocation_id: string;
  provider_id: string;
  provider_issue_id: string;
  source: string;
}

type View = "dashboard" | "members" | "invest" | "allotment" | "settings";
type BootState = "loading" | "ready" | "offline";

interface AllotmentCandidate {
  application_id: string;
  session_id: string;
  ipo_name: string;
  planned_amount_paise: number;
  account_count: number;
  registrar_id: string;
  registrar_name: string;
  provider_id: string;
  provider_name: string;
  official_status_url?: string | null;
  provider_health: string;
  expected_allotment_date?: string | null;
  pending_count: number;
  final_count: number;
  last_checked?: string | null;
  overall_job_state: string;
}

interface AllotmentReportRow {
  attempt_id: string;
  account_id: string;
  display_name: string;
  account_kind: string;
  masked_pan: string;
  status: string;
  allotted_lots?: number | null;
  allotted_shares?: number | null;
  provider_id: string;
  registrar_id: string;
  source: string;
  provenance: string;
  application_amount_paise: number;
  checked_at?: string | null;
  safe_provider_reference?: string | null;
  safe_message?: string | null;
  next_retry_at?: string | null;
  human_verification_state?: string | null;
  estimated_profit_paise?: number | null;
  profit_basis: string;
  profit_provenance?: string | null;
}

interface AllotmentJobReport {
  job_id: string;
  application_id: string;
  ipo_name: string;
  registrar_id: string;
  registrar_name: string;
  provider_id: string;
  status: string;
  official_status_url?: string | null;
  checked_at?: string | null;
  final_count: number;
  pending_count: number;
  accounts: AllotmentReportRow[];
}

interface SecurityStatus {
  mode: string;
  key_provider: string;
  real_pan_allowed: boolean;
  os_keyring_release_blocker: boolean;
  blocker?: string | null;
}

interface LookupAuthorizationStatus {
  provider_id: string;
  status: "NOT_GRANTED" | "ACTIVE" | "EXPIRED";
  authorization_id?: string | null;
  expiry_time?: string | null;
}

interface UpstoxConnectionStatus {
  provider: "UPSTOX_IPO_DATA";
  state: "NOT_CONNECTED" | "CONNECTED" | "FAILED";
  safe_message: string | null;
}

interface EstimatedProfit {
  basis: string;
  estimated_profit_paise?: number | null;
  note?: string | null;
  is_realized: boolean;
}

const EMPTY_DASHBOARD: DashboardData = {
  total_planned_paise: 0,
  submitted_session_count: 0,
  member_count: 0,
  friend_count: 0,
  profit_paise: 0,
};

const defaultBridge: CommandBridge = {
  invoke: (command, args) => tauriInvoke(command, args),
};

function newId(prefix: string): string {
  const value = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${value}`;
}

function paiseFromRupees(value: string): number {
  const rupees = Number(value);
  return Number.isFinite(rupees) ? Math.round(rupees * 100) : 0;
}

function formatRupees(paise: number): string {
  return new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency: "INR",
    maximumFractionDigits: 0,
  }).format(paise / 100);
}

function formatOptionalRupees(paise: number | null): string {
  return paise == null ? "TBA" : formatRupees(paise);
}

function rupeesInputFromPaise(paise: number | null): string {
  if (paise == null || paise <= 0) return "";
  return (paise / 100).toFixed(paise % 100 === 0 ? 0 : 2);
}

interface OfficialLotPlan {
  quantity: number;
  amountPerAccountPaise: number;
  totalCapitalPaise: number;
}

function calculateOfficialPlan(
  snapshot: IpoMetadataSnapshot,
  lots: number,
  accountCount: number,
): OfficialLotPlan | null {
  if (
    !Number.isSafeInteger(lots) ||
    lots <= 0 ||
    snapshot.lot_size == null ||
    snapshot.minimum_quantity == null ||
    snapshot.minimum_lots == null ||
    snapshot.planning_price_paise == null ||
    (snapshot.minimum_lots != null && lots < snapshot.minimum_lots)
  ) {
    return null;
  }
  const quantity = snapshot.lot_size * lots;
  const amountPerAccountPaise = quantity * snapshot.planning_price_paise;
  const totalCapitalPaise = amountPerAccountPaise * accountCount;
  if (
    !Number.isSafeInteger(quantity) ||
    !Number.isSafeInteger(amountPerAccountPaise) ||
    !Number.isSafeInteger(totalCapitalPaise)
  ) {
    return null;
  }
  return { quantity, amountPerAccountPaise, totalCapitalPaise };
}

function registrarIdFromDetails(item: IpoCatalogItem): string {
  switch (item.registrar_short_name) {
    case "KFintech":
      return "kfintech";
    case "Bigshare":
      return "bigshare";
    case "MUFG Intime":
      return "mufg_intime";
    default:
      return "";
  }
}

function metadataSnapshotFromDetails(
  item: IpoCatalogItem,
  lots: number,
  accountCount: number,
): IpoMetadataSnapshot | null {
  if (item.status !== "OPEN") return null;
  const snapshot = {
    metadata_source: "UPSTOX_IPO_API" as const,
    source_ipo_id: item.source_ipo_id,
    source_status: "OPEN" as const,
    source_name: item.name,
    source_symbol: item.symbol,
    source_isin: item.isin,
    fetched_at: item.fetched_at,
    revalidated_at: null,
    minimum_price_paise: item.minimum_price_paise,
    maximum_price_paise: item.maximum_price_paise,
    cut_off_price_paise: item.cut_off_price_paise,
    planning_price_paise: item.planning_price_paise,
    price_basis: item.price_basis,
    lot_size: item.lot_size,
    minimum_quantity: item.minimum_quantity,
    minimum_lots: item.minimum_lots,
    lots,
    quantity: 0,
    amount_per_account_paise: 0,
    total_capital_paise: 0,
    bidding_start_date: item.bidding_start_date,
    bidding_end_date: item.bidding_end_date,
    allotment_date: item.allotment_date,
    listing_date: item.listing_date,
    registrar_name: item.registrar_name,
    registrar_short_name: item.registrar_short_name,
    registrar_website: item.registrar_website,
  } satisfies IpoMetadataSnapshot;
  const plan = calculateOfficialPlan(snapshot, lots, accountCount);
  return plan
    ? {
        ...snapshot,
        quantity: plan.quantity,
        amount_per_account_paise: plan.amountPerAccountPaise,
        total_capital_paise: plan.totalCapitalPaise,
      }
    : null;
}

function userFacingError(cause: unknown, fallback: string): string {
  const message = cause instanceof Error ? cause.message : String(cause);
  return /__TAURI_INTERNALS__|invoke|undefined/i.test(message) ? fallback : message;
}

function sourceLabel(source: string): string {
  return source === "OWNER_HISTORICAL_ENTRY" ? "Historical record" : "Current application";
}

function provenanceLabel(provenance: string): string {
  return provenance === "MANUAL" ? "Manual result" : "Registrar result";
}

type StatusTone = "positive" | "negative" | "warning" | "info" | "unknown";

type StatusPresentation = {
  label: string;
  glyph: string;
  tone: StatusTone;
};

function statusPresentation(status: string): StatusPresentation {
  const presentations: Record<string, StatusPresentation> = {
    ALLOTTED: { label: "Allotted", glyph: "✓", tone: "positive" },
    NOT_ALLOTTED: { label: "Not Allotted", glyph: "−", tone: "unknown" },
    NOT_FOUND: { label: "Not Found", glyph: "?", tone: "warning" },
    NEEDS_HUMAN_VERIFICATION: {
      label: "Verification Required",
      glyph: "!",
      tone: "warning",
    },
    PROVIDER_UNAVAILABLE: { label: "Provider Unavailable", glyph: "!", tone: "warning" },
    RETRYABLE_ERROR: { label: "Retry Required", glyph: "↻", tone: "warning" },
    RATE_LIMITED: { label: "Rate Limited", glyph: "!", tone: "warning" },
    UNKNOWN: { label: "Unknown", glyph: "?", tone: "unknown" },
    UNCONFIRMED: { label: "Unconfirmed", glyph: "?", tone: "unknown" },
    PENDING: { label: "Pending", glyph: "○", tone: "info" },
    QUEUED: { label: "Queued", glyph: "○", tone: "info" },
    RUNNING: { label: "Running", glyph: "↻", tone: "info" },
    READY: { label: "Ready to check", glyph: "○", tone: "info" },
    NONE: { label: "Ready to check", glyph: "○", tone: "info" },
    COMPLETE: { label: "Complete", glyph: "✓", tone: "positive" },
    COMPLETE_WITH_UNCONFIRMED: {
      label: "Complete with unconfirmed results",
      glyph: "!",
      tone: "warning",
    },
    PARTIALLY_COMPLETE: { label: "Partially Complete", glyph: "◐", tone: "warning" },
    CANCELLED: { label: "Cancelled", glyph: "×", tone: "negative" },
    FAILED: { label: "Failed", glyph: "×", tone: "negative" },
    MANUAL_RESULT: { label: "Manual Result", glyph: "M", tone: "unknown" },
    MANUAL_FALLBACK_REQUIRED: {
      label: "Manual fallback required",
      glyph: "M",
      tone: "warning",
    },
  };
  return (
    presentations[status] ?? {
      label: status.replaceAll("_", " ").toLowerCase(),
      glyph: "•",
      tone: "unknown",
    }
  );
}

function StatusBadge({ status }: { status: string }) {
  const presentation = statusPresentation(status);
  return (
    <span className={`status-badge ${presentation.tone}`}>
      <span aria-hidden="true">{presentation.glyph}</span>
      <span>{presentation.label}</span>
    </span>
  );
}

/** Compact horizontal bar — ratio must be derived from real values only. */
function FinanceBar({
  label,
  valueLabel,
  ratio,
  tone = "accent",
}: {
  label: string;
  valueLabel: string;
  ratio: number;
  tone?: "accent" | "positive" | "warning" | "unknown" | "muted";
}) {
  const width = Number.isFinite(ratio) ? Math.max(0, Math.min(100, Math.round(ratio * 100))) : 0;
  return (
    <div className="finance-bar" role="img" aria-label={`${label}: ${valueLabel}`}>
      <div className="finance-bar-meta">
        <span>{label}</span>
        <strong>{valueLabel}</strong>
      </div>
      <div className="finance-bar-track" aria-hidden="true">
        <span className={`finance-bar-fill ${tone}`} style={{ width: `${width}%` }} />
      </div>
    </div>
  );
}

function ProgressMeter({ label, done, total }: { label: string; done: number; total: number }) {
  const safeTotal = Math.max(0, total);
  const safeDone = Math.max(0, Math.min(done, safeTotal || done));
  const width = safeTotal === 0 ? 0 : Math.round((safeDone / safeTotal) * 100);
  return (
    <div
      className="progress-meter"
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={safeTotal}
      aria-valuenow={safeDone}
    >
      <div className="finance-bar-meta">
        <span>{label}</span>
        <strong>
          {safeDone}/{safeTotal || 0}
        </strong>
      </div>
      <div className="finance-bar-track" aria-hidden="true">
        <span className="finance-bar-fill accent" style={{ width: `${width}%` }} />
      </div>
    </div>
  );
}

function StatusRing({ buckets, total }: { buckets: Array<[string, number]>; total: number }) {
  const colors: Record<StatusTone, string> = {
    positive: "var(--positive)",
    negative: "var(--danger)",
    warning: "var(--warning)",
    info: "var(--accent)",
    unknown: "var(--unknown)",
  };
  let offset = 0;
  const segments = buckets.map(([status, count]) => {
    const start = offset;
    offset += total === 0 ? 0 : (count / total) * 100;
    const presentation = statusPresentation(status);
    return `${colors[presentation.tone]} ${start}% ${offset}%`;
  });
  const background =
    total === 0 ? "conic-gradient(var(--border) 0 100%)" : `conic-gradient(${segments.join(", ")})`;

  return (
    <div className="status-ring-layout">
      <div
        aria-label={`${total} application status records`}
        className="status-ring"
        role="img"
        style={{ background }}
      >
        <div className="status-ring-center">
          <strong>{total}</strong>
          <span>records</span>
        </div>
      </div>
      <ul className="status-legend">
        {buckets.length === 0 ? (
          <li className="status-legend-empty">No allotment checks yet</li>
        ) : (
          buckets.map(([status, count]) => {
            const presentation = statusPresentation(status);
            return (
              <li key={status}>
                <span className={`legend-dot ${presentation.tone}`} aria-hidden="true" />
                <span>{presentation.label}</span>
                <strong>{count}</strong>
              </li>
            );
          })
        )}
      </ul>
    </div>
  );
}

function Mark() {
  return (
    <span className="brand-mark" aria-hidden="true">
      S
    </span>
  );
}

function AppShell({
  active,
  setActive,
  children,
  memberCount,
}: {
  active: View;
  setActive: (view: View) => void;
  children: React.ReactNode;
  memberCount: number;
}) {
  const items: Array<{ label: string; view: View; glyph: string }> = [
    { label: "Dashboard", view: "dashboard", glyph: "▦" },
    { label: "Investments", view: "invest", glyph: "₹" },
    { label: "Check Allotment", view: "allotment", glyph: "✓" },
    { label: "Members", view: "members", glyph: "◎" },
    { label: "Settings", view: "settings", glyph: "⚙" },
  ];

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-row">
          <Mark />
          <div>
            <strong>Sanket IPO</strong>
            <span>Private investment ledger</span>
          </div>
        </div>
        <nav aria-label="Primary navigation">
          <p className="nav-label">Workspace</p>
          <ul>
            {items.map((item) => (
              <li key={item.label}>
                <button
                  aria-current={active === item.view ? "page" : undefined}
                  className={active === item.view ? "nav-item active" : "nav-item"}
                  onClick={() => setActive(item.view)}
                  type="button"
                >
                  <span className="nav-glyph" aria-hidden="true">
                    {item.glyph}
                  </span>
                  {item.label}
                </button>
              </li>
            ))}
          </ul>
        </nav>
        <div className="sidebar-footer">
          <div className="privacy-card">
            <strong>Local identity protected</strong>
            <span>Only masked PAN is shown outside the encrypted vault.</span>
          </div>
          <div className="profile-button">
            <span className="avatar" aria-hidden="true">
              {memberCount || "0"}
            </span>
            <span>
              <strong>
                {memberCount
                  ? `${memberCount} private profile${memberCount > 1 ? "s" : ""}`
                  : "No profile"}
              </strong>
              <small>Saved on this device</small>
            </span>
          </div>
        </div>
      </aside>
      <main className="app-main">
        <header className="topbar">
          <div className="topbar-status">
            <span className="saved-status">
              <span aria-hidden="true" /> Saved locally
            </span>
            <span className="sync-status">Pending sync</span>
          </div>
          <span className="topbar-seal">PRIVATE / LOCAL / AUDITED</span>
        </header>
        {children}
      </main>
    </div>
  );
}

function SettingsView({ bridge }: { bridge: CommandBridge }) {
  const [status, setStatus] = useState<UpstoxConnectionStatus | null>(null);
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const loadStatus = useCallback(async () => {
    try {
      const next = await bridge.invoke<UpstoxConnectionStatus>("get_upstox_connection_status");
      setStatus(next);
      setError("");
    } catch {
      setStatus({
        provider: "UPSTOX_IPO_DATA",
        state: "FAILED",
        safe_message: "Could not read the OS keyring.",
      });
    }
  }, [bridge]);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  async function saveToken(event: FormEvent) {
    event.preventDefault();
    if (!token.trim()) {
      setToken("");
      setError("Enter an Analytics Token.");
      return;
    }
    setBusy(true);
    setError("");
    let submittedToken = token;
    setToken("");
    try {
      const command =
        status?.state === "CONNECTED"
          ? "replace_upstox_analytics_token"
          : "connect_upstox_analytics_token";
      const next = await bridge.invoke<UpstoxConnectionStatus>(command, {
        request: { token: submittedToken },
      });
      setStatus(next);
      if (next.state === "FAILED") setError(next.safe_message ?? "Could not save the token.");
    } catch {
      setError("Could not save the Analytics Token.");
    } finally {
      submittedToken = "";
      setToken("");
      setBusy(false);
    }
  }

  async function disconnectProvider() {
    setBusy(true);
    setError("");
    try {
      setStatus(await bridge.invoke<UpstoxConnectionStatus>("disconnect_upstox"));
    } catch {
      setError("Could not disconnect Upstox.");
    } finally {
      setBusy(false);
    }
  }

  const connected = status?.state === "CONNECTED";
  return (
    <div className="content-column settings-view">
      <header className="page-heading">
        <div>
          <span className="eyebrow">Settings / Data Sources</span>
          <h1>Data Sources</h1>
          <p>Connect read-only public IPO metadata without sharing private Sanket data.</p>
        </div>
      </header>
      <section className="form-panel provider-panel" aria-labelledby="upstox-heading">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Read-only provider</span>
            <h2 id="upstox-heading">Upstox IPO Data</h2>
          </div>
          <span className={`status-badge ${connected ? "positive" : "warning"}`}>
            {connected ? "Connected" : status?.state === "FAILED" ? "Unavailable" : "Not connected"}
          </span>
        </div>
        <div className="provider-panel-body">
          <p>
            Use the Analytics Token from Upstox Developer Apps → Analytics. Sanket stores it only in
            this device&apos;s OS keyring. No OAuth flow or network validation is performed here.
          </p>
          <form onSubmit={saveToken}>
            <label htmlFor="upstox-token">Upstox Analytics Token</label>
            <input
              id="upstox-token"
              type="password"
              autoComplete="off"
              maxLength={4096}
              value={token}
              onChange={(event) => setToken(event.target.value)}
              disabled={busy}
            />
            <div className="provider-actions">
              <button className="primary-button" type="submit" disabled={busy || !token.trim()}>
                {connected ? "Replace Upstox" : "Connect Upstox"}
              </button>
              {connected && (
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => void disconnectProvider()}
                  disabled={busy}
                >
                  Disconnect
                </button>
              )}
              <button
                className="text-button"
                type="button"
                onClick={() => void loadStatus()}
                disabled={busy}
              >
                Refresh status
              </button>
            </div>
          </form>
          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}
          {status?.safe_message && status.state === "FAILED" && (
            <p className="form-error" role="alert">
              {status.safe_message}
            </p>
          )}
          <p className="provider-boundary-note">
            Token values are never returned to the app UI, stored in SQLite, or sent to Upstox in
            this phase.
          </p>
        </div>
      </section>
    </div>
  );
}

function Onboarding({
  bridge,
  onCreated,
}: {
  bridge: CommandBridge;
  onCreated: (member: MemberRow) => void;
}) {
  const [form, setForm] = useState({
    name: "",
    email: "",
    account: "",
    broker: "",
    upi: "",
    pan: "",
    consented: false,
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const update = (key: keyof typeof form, value: string | boolean) =>
    setForm((current) => ({ ...current, [key]: value }));

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!form.consented) {
      setError("Acknowledge the local identity-storage notice before creating the profile.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      const memberId = newId("member");
      const response = await bridge.invoke<{ member_id: string; masked_pan: string }>(
        "onboard_member",
        {
          request: {
            member_id: memberId,
            display_name: form.name,
            email: form.email,
            role: "OWNER",
            primary_account_label: form.account,
            broker: form.broker,
            upi_id: form.upi,
            pan: form.pan,
            consented: form.consented,
          },
        },
      );
      setForm((current) => ({ ...current, pan: "", upi: "" }));
      onCreated({
        id: response.member_id,
        name: form.name,
        role: "OWNER",
        masked_pan: response.masked_pan,
      });
    } catch (cause) {
      setError(
        userFacingError(
          cause,
          "The owner profile could not be saved. Reopen the app and try again.",
        ),
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="onboarding-page">
      <section className="onboarding-intro">
        <div className="brand-row onboarding-brand">
          <Mark />
          <div>
            <strong>Sanket IPO</strong>
            <span>Private group OS</span>
          </div>
        </div>
        <p className="eyebrow">First-run setup</p>
        <h1>Set up the private member vault</h1>
        <p>
          This profile becomes the owner account for local investment sessions. Identity values are
          validated and encrypted by Rust before they reach disk.
        </p>
        <dl className="privacy-ledger">
          <div>
            <dt>Identity</dt>
            <dd>Encrypted on this device</dd>
          </div>
          <div>
            <dt>AI boundary</dt>
            <dd>No member names, PAN, UPI, or proofs</dd>
          </div>
          <div>
            <dt>Sync</dt>
            <dd>Local operation never waits for Git</dd>
          </div>
        </dl>
      </section>
      <form className="form-panel onboarding-form" onSubmit={submit}>
        <p className="eyebrow">Owner profile</p>
        <div className="security-strip ready" role="status">
          <strong>Protected identity fields</strong>
          <span>PAN and UPI go directly to the local Rust vault and never to AI requests.</span>
        </div>
        <div className="form-grid two-column">
          <label>
            Full name
            <input required value={form.name} onChange={(e) => update("name", e.target.value)} />
          </label>
          <label>
            Email
            <input
              required
              type="email"
              value={form.email}
              onChange={(e) => update("email", e.target.value)}
            />
          </label>
          <label>
            Primary demat account
            <input
              required
              value={form.account}
              onChange={(e) => update("account", e.target.value)}
            />
          </label>
          <label>
            Broker
            <input
              required
              value={form.broker}
              onChange={(e) => update("broker", e.target.value)}
            />
          </label>
          <label>
            UPI ID
            <input required value={form.upi} onChange={(e) => update("upi", e.target.value)} />
          </label>
          <label>
            PAN
            <input
              autoCapitalize="characters"
              autoComplete="off"
              maxLength={10}
              required
              value={form.pan}
              onChange={(e) => update("pan", e.target.value.toUpperCase())}
            />
          </label>
        </div>
        <label className="consent-row">
          <input
            checked={form.consented}
            onChange={(e) => update("consented", e.target.checked)}
            type="checkbox"
          />
          <span>
            I understand PAN and UPI are encrypted locally and never included in AI requests.
          </span>
        </label>
        {error && (
          <p className="inline-error" role="alert">
            {error}
          </p>
        )}
        <button className="primary-button" disabled={busy} type="submit">
          {busy ? "Creating encrypted profile…" : "Create private profile"}
        </button>
      </form>
    </main>
  );
}

function DashboardView({
  bridge,
  dashboard,
  ownerMemberId,
  onInvest,
  onAllotment,
  onChanged,
}: {
  bridge: CommandBridge;
  dashboard: DashboardData;
  ownerMemberId?: string;
  onInvest: () => void;
  onAllotment: () => void;
  onChanged?: () => void | Promise<void>;
}) {
  const [activity, setActivity] = useState<AllotmentCandidate[]>([]);
  const [activityError, setActivityError] = useState(false);
  const [voidTarget, setVoidTarget] = useState<AllotmentCandidate | null>(null);
  const [voidReason, setVoidReason] = useState(
    "Accidental current entry — correcting before historical record",
  );
  const [voidAffirmed, setVoidAffirmed] = useState(false);
  const [voidBusy, setVoidBusy] = useState(false);
  const [voidError, setVoidError] = useState("");
  const [voidMessage, setVoidMessage] = useState("");

  const reloadActivity = useCallback(() => {
    bridge
      .invoke<AllotmentCandidate[]>("list_allotment_candidates")
      .then(setActivity)
      .catch(() => setActivityError(true));
  }, [bridge]);

  useEffect(() => {
    let active = true;
    bridge
      .invoke<AllotmentCandidate[]>("list_allotment_candidates")
      .then((rows) => {
        if (active) setActivity(rows);
      })
      .catch(() => {
        if (active) setActivityError(true);
      });
    return () => {
      active = false;
    };
  }, [bridge]);

  async function confirmVoid() {
    if (!voidTarget || !ownerMemberId) return;
    setVoidBusy(true);
    setVoidError("");
    setVoidMessage("");
    try {
      await bridge.invoke("void_submitted_session", {
        request: {
          session_id: voidTarget.session_id,
          actor_member_id: ownerMemberId,
          reason: voidReason.trim(),
          owner_affirmed: voidAffirmed,
        },
      });
      setVoidMessage(`Session voided · ${voidTarget.ipo_name}`);
      setVoidTarget(null);
      setVoidAffirmed(false);
      reloadActivity();
      await onChanged?.();
    } catch (cause) {
      setVoidError(userFacingError(cause, "The submitted entry could not be voided. Try again."));
    } finally {
      setVoidBusy(false);
    }
  }

  const metrics = [
    {
      label: "Total invested",
      value: formatRupees(dashboard.total_planned_paise),
      detail: `${dashboard.submitted_session_count} submitted session${dashboard.submitted_session_count === 1 ? "" : "s"}`,
    },
    {
      label: "Realized profit",
      value: formatRupees(dashboard.profit_paise),
      detail: "Only recorded allotment outcomes",
    },
    {
      label: "Applications",
      value: String(activity.length),
      detail: "Submitted records available to check",
    },
    {
      label: "Accounts",
      value: String(dashboard.member_count + dashboard.friend_count),
      detail: `${dashboard.member_count} core / ${dashboard.friend_count} friend`,
    },
  ];

  const accountTotal = dashboard.member_count + dashboard.friend_count;
  const capitalScale = Math.max(dashboard.total_planned_paise, dashboard.profit_paise, 1);
  const exposureMax = Math.max(...activity.map((row) => row.planned_amount_paise), 0);
  const statusBuckets = useMemo(() => {
    const counts = new Map<string, number>();
    for (const row of activity) {
      counts.set(row.overall_job_state, (counts.get(row.overall_job_state) ?? 0) + 1);
    }
    return [...counts.entries()].sort((left, right) => right[1] - left[1]);
  }, [activity]);
  const pendingAccounts = activity.reduce((sum, row) => sum + row.pending_count, 0);
  const finalAccounts = activity.reduce((sum, row) => sum + row.final_count, 0);
  const checkTotal = pendingAccounts + finalAccounts;

  return (
    <div className="dashboard page-content">
      <header className="page-heading dashboard-heading">
        <div>
          <h1>Dashboard</h1>
          <p>Private IPO portfolio overview</p>
        </div>
        <fieldset className="quick-actions" aria-label="Quick actions">
          <button
            aria-label="Check Allotment"
            className="secondary-button action-with-note"
            onClick={onAllotment}
            type="button"
          >
            <span>Check allotment</span>
            <small>Registrar workflow</small>
          </button>
          <button
            aria-label="Invest"
            className="primary-button action-with-note"
            onClick={onInvest}
            type="button"
          >
            <span>Invest</span>
            <small>Preview, edit, record</small>
          </button>
        </fieldset>
      </header>

      <section className="summary-strip kpi-grid" aria-label="Portfolio summary">
        {metrics.map((metric) => (
          <div className="summary-metric kpi-card" key={metric.label}>
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
            <small>{metric.detail}</small>
          </div>
        ))}
      </section>

      <section className="finance-grid dashboard-analytics" aria-label="Portfolio analytics">
        <article className="panel finance-panel capital-panel">
          <div className="panel-heading compact-heading">
            <div>
              <h2>Capital overview</h2>
              <p>Active capital and realized outcomes</p>
            </div>
          </div>
          <div className="capital-highlight">
            <div>
              <span>Active capital</span>
              <strong>{formatRupees(dashboard.total_planned_paise)}</strong>
            </div>
            <div className="capital-highlight-side">
              <span>Realized profit</span>
              <strong className="positive-value">{formatRupees(dashboard.profit_paise)}</strong>
            </div>
          </div>
          <div className="finance-stack">
            <FinanceBar
              label="Capital deployed"
              valueLabel={formatRupees(dashboard.total_planned_paise)}
              ratio={dashboard.total_planned_paise / capitalScale}
              tone="accent"
            />
            <FinanceBar
              label="Realized profit"
              valueLabel={formatRupees(dashboard.profit_paise)}
              ratio={dashboard.profit_paise / capitalScale}
              tone="positive"
            />
            <FinanceBar
              label="Core accounts"
              valueLabel={`${dashboard.member_count}`}
              ratio={accountTotal === 0 ? 0 : dashboard.member_count / accountTotal}
              tone="accent"
            />
            <FinanceBar
              label="Friend accounts"
              valueLabel={`${dashboard.friend_count}`}
              ratio={accountTotal === 0 ? 0 : dashboard.friend_count / accountTotal}
              tone="unknown"
            />
          </div>
        </article>

        <article className="panel finance-panel status-panel">
          <div className="panel-heading compact-heading">
            <div>
              <h2>Allotment status</h2>
              <p>Application-level checks, kept distinct</p>
            </div>
          </div>
          {activity.length === 0 ? (
            <>
              <StatusRing buckets={statusBuckets} total={activity.length} />
              <div className="finance-empty compact-empty">
                No allotment checks yet. Status will appear after a submitted application is
                checked.
              </div>
            </>
          ) : (
            <div className="status-panel-body">
              <StatusRing buckets={statusBuckets} total={activity.length} />
              <ProgressMeter label="Accounts finalized" done={finalAccounts} total={checkTotal} />
              {statusBuckets.map(([status, count]) => {
                const presentation = statusPresentation(status);
                const tone =
                  presentation.tone === "positive"
                    ? "positive"
                    : presentation.tone === "warning" || presentation.tone === "negative"
                      ? "warning"
                      : presentation.tone === "unknown"
                        ? "unknown"
                        : "accent";
                return (
                  <FinanceBar
                    key={status}
                    label={presentation.label}
                    valueLabel={`${count}`}
                    ratio={count / activity.length}
                    tone={tone}
                  />
                );
              })}
            </div>
          )}
        </article>

        <article className="panel finance-panel finance-panel-wide">
          <div className="panel-heading compact-heading">
            <div>
              <h2>IPO exposure</h2>
              <p>Planned amounts by submitted application</p>
            </div>
            <span>{activity.length} records</span>
          </div>
          {activity.length === 0 ? (
            <div className="finance-empty">
              No active exposure yet. Start an investment or add a historical application.
            </div>
          ) : (
            <div className="finance-stack">
              {activity.map((candidate) => (
                <FinanceBar
                  key={candidate.application_id}
                  label={candidate.ipo_name}
                  valueLabel={formatRupees(candidate.planned_amount_paise)}
                  ratio={exposureMax === 0 ? 0 : candidate.planned_amount_paise / exposureMax}
                  tone="accent"
                />
              ))}
            </div>
          )}
        </article>
      </section>

      <section className="dashboard-grid">
        <article className="panel activity-panel">
          <div className="panel-heading compact-heading">
            <div>
              <h2>Current activity</h2>
              <p>Application queue</p>
            </div>
            <span>{activity.length} records</span>
          </div>
          <div className="table-scroll">
            <table className="data-table">
              <thead>
                <tr>
                  <th>IPO</th>
                  <th>Registrar</th>
                  <th>Status</th>
                  <th className="numeric">Amount</th>
                  <th>Last checked</th>
                  <th>Correction</th>
                </tr>
              </thead>
              <tbody>
                {activity.map((candidate) => (
                  <tr key={candidate.application_id}>
                    <th scope="row">{candidate.ipo_name}</th>
                    <td>{candidate.registrar_name}</td>
                    <td>
                      <StatusBadge status={candidate.overall_job_state} />
                    </td>
                    <td className="numeric">{formatRupees(candidate.planned_amount_paise)}</td>
                    <td>{candidate.last_checked ?? "Not checked"}</td>
                    <td>
                      <button
                        className="text-button"
                        disabled={!ownerMemberId || voidBusy}
                        onClick={() => {
                          setVoidTarget(candidate);
                          setVoidError("");
                          setVoidMessage("");
                          setVoidAffirmed(false);
                        }}
                        type="button"
                      >
                        Void entry
                      </button>
                    </td>
                  </tr>
                ))}
                {activity.length === 0 && (
                  <tr className="table-empty">
                    <td colSpan={6}>
                      {activityError
                        ? "Application activity is unavailable. Core ledger totals remain available."
                        : "No submitted applications. Start an investment or add a historical record."}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          {voidMessage && (
            <p className="inline-ok" role="status">
              {voidMessage}
            </p>
          )}
          {voidTarget && (
            <form
              aria-label="Void submitted session"
              className="void-form form-panel"
              onSubmit={(event) => {
                event.preventDefault();
                void confirmVoid();
              }}
            >
              <div className="security-strip blocker" role="status">
                <strong>Owner correction</strong>
                <span>
                  Voids “{voidTarget.ipo_name}” ({formatRupees(voidTarget.planned_amount_paise)}).
                  Original events stay auditable. Active totals and allotment eligibility drop.
                </span>
              </div>
              <label>
                Reason
                <input
                  maxLength={200}
                  required
                  value={voidReason}
                  onChange={(event) => setVoidReason(event.target.value)}
                />
              </label>
              <label className="check-row">
                <input
                  checked={voidAffirmed}
                  onChange={(event) => setVoidAffirmed(event.target.checked)}
                  type="checkbox"
                />
                <span>I affirm this submitted entry should be voided as an owner correction.</span>
              </label>
              {voidError && (
                <p className="inline-error" role="alert">
                  {voidError}
                </p>
              )}
              <div className="action-buttons">
                <button
                  className="secondary-button"
                  disabled={voidBusy}
                  onClick={() => setVoidTarget(null)}
                  type="button"
                >
                  Cancel
                </button>
                <button
                  className="primary-button"
                  disabled={voidBusy || !voidAffirmed}
                  type="submit"
                >
                  {voidBusy ? "Voiding…" : "Confirm void"}
                </button>
              </div>
            </form>
          )}
        </article>

        <aside className="panel quick-panel">
          <div className="panel-heading compact-heading">
            <div>
              <h2>Quick actions</h2>
              <p>Owner controls</p>
            </div>
          </div>
          <button className="quick-row" onClick={onInvest} type="button">
            <span className="quick-glyph" aria-hidden="true">
              ₹
            </span>
            <span>
              <strong>New investment</strong>
              <small>Preview a recommendation, then record the decision.</small>
            </span>
            <span aria-hidden="true">›</span>
          </button>
          <button className="quick-row" onClick={onInvest} type="button">
            <span className="quick-glyph" aria-hidden="true">
              H
            </span>
            <span>
              <strong>Historical application</strong>
              <small>Owner-entered fact with no inferred result.</small>
            </span>
            <span aria-hidden="true">›</span>
          </button>
          <button className="quick-row" onClick={onAllotment} type="button">
            <span className="quick-glyph" aria-hidden="true">
              ✓
            </span>
            <span>
              <strong>Check allotment</strong>
              <small>Continue through the mapped registrar workflow.</small>
            </span>
            <span aria-hidden="true">›</span>
          </button>
        </aside>
      </section>

      <section className="decision-boundary" aria-label="Investment decision boundary">
        <div>
          <strong>CHECK: recommendation preview</strong>
          <p>Uses approved non-secret inputs. It does not record an investment decision.</p>
        </div>
        <span className="decision-arrow" aria-hidden="true">
          →
        </span>
        <div className="record-boundary">
          <strong>SUBMIT: record investment</strong>
          <p>Persists the human-selected accounts and amounts into the private event ledger.</p>
        </div>
      </section>
    </div>
  );
}

function MembersView({
  bridge,
  members,
  friends,
  onFriendAdded,
  onFriendArchived,
}: {
  bridge: CommandBridge;
  members: MemberRow[];
  friends: FriendRow[];
  onFriendAdded: (friend: FriendRow) => void;
  onFriendArchived: (id: string) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [form, setForm] = useState({ name: "", upi: "", pan: "", broker: "", eligible: true });
  const [error, setError] = useState("");
  const owner = members[0];

  async function submit(event: FormEvent) {
    event.preventDefault();
    setError("");
    try {
      const friendId = newId("friend");
      const response = await bridge.invoke<{ friend_id: string; masked_pan: string }>(
        "add_friend",
        {
          request: {
            friend_id: friendId,
            owner_member_id: owner.id,
            name: form.name,
            upi_id: form.upi,
            pan: form.pan,
            broker: form.broker,
            share_eligible: form.eligible,
            share_basis_points: form.eligible ? 1000 : 0,
          },
        },
      );
      onFriendAdded({
        id: response.friend_id,
        owner_member_id: owner.id,
        label: form.name,
        masked_pan: response.masked_pan,
        share_basis_points: form.eligible ? 1000 : 0,
      });
      setForm({ name: "", upi: "", pan: "", broker: "", eligible: true });
      setAdding(false);
    } catch (cause) {
      setError(userFacingError(cause, "The request could not be completed. Try again."));
    }
  }

  async function archive(friend: FriendRow) {
    await bridge.invoke("archive_friend", {
      friendId: friend.id,
      ownerMemberId: friend.owner_member_id,
    });
    onFriendArchived(friend.id);
  }

  return (
    <div className="page-content members-page">
      <section className="section-heading">
        <div>
          <p className="eyebrow">Private accounts</p>
          <h1>Members and friend accounts</h1>
          <p>Only masked identity metadata is visible outside the encrypted vault.</p>
        </div>
        <button
          aria-controls="friend-account-form"
          aria-expanded={adding}
          className="primary-button"
          onClick={() => setAdding((value) => !value)}
          type="button"
        >
          {adding ? "Close form" : "Add friend account"}
        </button>
      </section>

      {adding && (
        <form className="form-panel friend-form" id="friend-account-form" onSubmit={submit}>
          <div className="security-strip ready" role="status">
            <strong>Encrypted intake</strong>
            <span>Plaintext identity is accepted only for this local vault operation.</span>
          </div>
          <div className="form-grid four-column">
            <label>
              Friend name
              <input
                required
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
              />
            </label>
            <label>
              UPI ID
              <input
                required
                value={form.upi}
                onChange={(e) => setForm({ ...form, upi: e.target.value })}
              />
            </label>
            <label>
              PAN
              <input
                autoComplete="off"
                maxLength={10}
                required
                value={form.pan}
                onChange={(e) => setForm({ ...form, pan: e.target.value.toUpperCase() })}
              />
            </label>
            <label>
              Broker
              <input
                required
                value={form.broker}
                onChange={(e) => setForm({ ...form, broker: e.target.value })}
              />
            </label>
          </div>
          <label className="consent-row compact">
            <input
              checked={form.eligible}
              onChange={(e) => setForm({ ...form, eligible: e.target.checked })}
              type="checkbox"
            />
            <span>Eligible for 10% profit share</span>
          </label>
          {error && (
            <p className="inline-error" role="alert">
              {error}
            </p>
          )}
          <button className="primary-button" type="submit">
            Encrypt and add friend
          </button>
        </form>
      )}

      <section className="account-list" aria-label="Core members">
        <p className="eyebrow">Core members</p>
        <div className="member-summary panel member-overview">
          <div className="member-overview-copy">
            <span>Account map</span>
            <p>Core members belong to this workspace. Friend accounts are shared for review.</p>
          </div>
          <fieldset className="member-count-grid">
            <legend className="sr-only">Account counts</legend>
            <div>
              <strong>{members.length}</strong>
              <span>Core members</span>
            </div>
            <div>
              <strong>{friends.length}</strong>
              <span>Friend accounts</span>
            </div>
          </fieldset>
        </div>
        {members.map((member) => (
          <article className="account-row" key={member.id}>
            <span className="account-monogram">{member.name.slice(0, 2).toUpperCase()}</span>
            <div>
              <strong>{member.name}</strong>
              <small>{member.role.replace("_", " ")}</small>
            </div>
            <code>{member.masked_pan}</code>
            <span className="status-flag">ACTIVE</span>
          </article>
        ))}
      </section>

      <section className="account-list" aria-label="Friend accounts">
        <p className="eyebrow">Friend accounts</p>
        {friends.length === 0 ? (
          <div className="empty-row">
            <span>No friend accounts yet. Add one without exposing plaintext PAN.</span>
            <button className="text-button" onClick={() => setAdding(true)} type="button">
              + Add friend account
            </button>
          </div>
        ) : (
          friends.map((friend) => (
            <article className="account-row" key={friend.id}>
              <span className="account-monogram friend">
                {friend.label.slice(0, 2).toUpperCase()}
              </span>
              <div>
                <strong>{friend.label}</strong>
                <small>{friend.share_basis_points / 100}% profit share</small>
                <FinanceBar
                  label="Profit share"
                  valueLabel={`${friend.share_basis_points / 100}%`}
                  ratio={friend.share_basis_points / 10000}
                  tone="positive"
                />
              </div>
              <code>{friend.masked_pan}</code>
              <button className="text-button" onClick={() => void archive(friend)} type="button">
                Archive
              </button>
            </article>
          ))
        )}
      </section>
    </div>
  );
}

function HistoricalApplicationForm({
  bridge,
  members,
  onCancel,
  onSubmitted,
}: {
  bridge: CommandBridge;
  members: MemberRow[];
  onCancel: () => void;
  onSubmitted: () => Promise<void>;
}) {
  const owner = members.find((member) => member.role === "OWNER");
  const [ipoName, setIpoName] = useState("");
  const [amountRupees, setAmountRupees] = useState("");
  const [applicationDate, setApplicationDate] = useState("");
  const [accountId, setAccountId] = useState(owner?.id ?? "");
  const [registrarId, setRegistrarId] = useState("mufg_intime");
  const [providerIssueId, setProviderIssueId] = useState("");
  const [affirmed, setAffirmed] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!owner || accountId !== owner.id) {
      setError("Select the existing owner primary account.");
      return;
    }
    setSaving(true);
    setError("");
    setMessage("");
    try {
      const response = await bridge.invoke<HistoricalApplicationResponse>(
        "record_historical_application",
        {
          request: {
            actor_member_id: owner.id,
            account_id: accountId,
            ipo_name: ipoName.trim(),
            amount_paise: paiseFromRupees(amountRupees),
            application_date: applicationDate || null,
            registrar_id: registrarId,
            provider_issue_id: providerIssueId.trim(),
            owner_affirmed: affirmed,
          },
        },
      );
      setMessage(`Historical application saved · ${response.application_id}`);
      await onSubmitted();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="page-content invest-page">
      <section className="section-heading invest-heading">
        <div>
          <p className="eyebrow">Owner historical entry</p>
          <h1>Add historical application</h1>
          <p>Records an owner-affirmed fact. It does not contact the registrar or set a result.</p>
          <span className="status-badge history">
            <span aria-hidden="true">H</span>
            Owner entered / no registrar lookup
          </span>
        </div>
        <button className="secondary-button" type="button" onClick={onCancel}>
          Back to current investment
        </button>
      </section>
      <form className="form-panel investment-form" onSubmit={save}>
        <label>
          Historical IPO name
          <input value={ipoName} onChange={(event) => setIpoName(event.target.value)} required />
        </label>
        <label>
          Historical application amount (₹)
          <input
            inputMode="decimal"
            value={amountRupees}
            onChange={(event) => setAmountRupees(event.target.value)}
            required
          />
        </label>
        <label>
          Application date (optional)
          <input
            aria-label="Historical application date"
            type="date"
            value={applicationDate}
            onChange={(event) => setApplicationDate(event.target.value)}
          />
        </label>
        <label>
          Historical application account
          <select
            aria-label="Historical application account"
            value={accountId}
            onChange={(event) => setAccountId(event.target.value)}
            required
          >
            <option value="">Select owner account</option>
            {owner && <option value={owner.id}>{owner.name} · primary account</option>}
          </select>
        </label>
        <label>
          Registrar
          <select value={registrarId} onChange={(event) => setRegistrarId(event.target.value)}>
            <option value="kfintech">KFintech</option>
            <option value="bigshare">Bigshare Services</option>
            <option value="mufg_intime">MUFG Intime India Private Limited</option>
          </select>
        </label>
        <label>
          Provider issue ID
          <input
            value={providerIssueId}
            onChange={(event) => setProviderIssueId(event.target.value)}
            required
          />
        </label>
        <label className="check-row">
          <input
            type="checkbox"
            checked={affirmed}
            onChange={(event) => setAffirmed(event.target.checked)}
            required
          />
          I affirm this is a real historical application made from my primary account.
        </label>
        <p>
          Owner-entered record · No automated result — this record is not checked with the
          registrar.
        </p>
        {message && (
          <p className="success-message" role="status">
            {message}
          </p>
        )}
        {error && (
          <p className="error-message" role="alert">
            {error}
          </p>
        )}
        <button className="primary-button" type="submit" disabled={saving || !owner}>
          {saving ? "Saving…" : "Save historical application"}
        </button>
      </form>
    </div>
  );
}

function InvestView({
  bridge,
  members,
  friends,
  onSubmitted,
}: {
  bridge: CommandBridge;
  members: MemberRow[];
  friends: FriendRow[];
  onSubmitted: () => Promise<void>;
}) {
  const sessionId = useRef(newId("session"));
  const [historicalMode, setHistoricalMode] = useState(false);
  const [dailyRupees, setDailyRupees] = useState("");
  const [ipos, setIpos] = useState<IpoDraft[]>([
    {
      id: newId("ipo"),
      name: "",
      amountRupees: "",
      registrarId: "kfintech",
      expectedAllotmentDate: "",
      included: true,
      lots: "",
      metadataSnapshot: null,
    },
  ]);
  const [accountIds, setAccountIds] = useState<string[]>([]);
  const [recommendation, setRecommendation] = useState<CheckResponse | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [openCatalogue, setOpenCatalogue] = useState<IpoCatalogue | null>(null);
  const [upcomingCatalogue, setUpcomingCatalogue] = useState<IpoCatalogue | null>(null);
  const [selectedDetails, setSelectedDetails] = useState<IpoCatalogItem | null>(null);
  const [catalogueError, setCatalogueError] = useState("");
  const [catalogueBusy, setCatalogueBusy] = useState(false);
  const [revalidationPending, setRevalidationPending] = useState(false);
  const accounts = useMemo(
    () => [
      ...members.map((member) => ({ id: member.id, label: member.name, pan: member.masked_pan })),
      ...friends.map((friend) => ({ id: friend.id, label: friend.label, pan: friend.masked_pan })),
    ],
    [members, friends],
  );

  function officialPlanFor(ipo: IpoDraft): OfficialLotPlan | null {
    if (!ipo.metadataSnapshot) return null;
    return calculateOfficialPlan(ipo.metadataSnapshot, Number(ipo.lots), accountIds.length);
  }

  function amountPaiseFor(ipo: IpoDraft): number {
    return ipo.metadataSnapshot
      ? (officialPlanFor(ipo)?.amountPerAccountPaise ?? 0)
      : paiseFromRupees(ipo.amountRupees);
  }

  const calculatedTotalCapital = (() => {
    const selected = ipos.filter((ipo) => ipo.included);
    if (selected.length === 0 || selected.some((ipo) => !ipo.metadataSnapshot)) return null;
    const plans = selected.map(officialPlanFor);
    if (!plans.every((plan): plan is OfficialLotPlan => plan !== null)) return null;
    const total = plans.reduce((sum, plan) => sum + plan.totalCapitalPaise, 0);
    return Number.isSafeInteger(total) ? total : null;
  })();

  useEffect(() => {
    let active = true;
    setCatalogueBusy(true);
    Promise.all([
      bridge.invoke<IpoCatalogue>("list_available_ipos", { query: { status: "OPEN" } }),
      bridge.invoke<IpoCatalogue>("list_available_ipos", { query: { status: "UPCOMING" } }),
    ])
      .then(([open, upcoming]) => {
        if (!active) return;
        setOpenCatalogue(open);
        setUpcomingCatalogue(upcoming);
        setCatalogueError("");
      })
      .catch(() => {
        if (active)
          setCatalogueError("Upstox is unavailable. Manual entry remains available below.");
      })
      .finally(() => {
        if (active) setCatalogueBusy(false);
      });
    return () => {
      active = false;
    };
  }, [bridge]);

  function updateIpo(id: string, patch: Partial<IpoDraft>) {
    setIpos((current) => current.map((ipo) => (ipo.id === id ? { ...ipo, ...patch } : ipo)));
    setRecommendation(null);
    setMessage("");
    setRevalidationPending(false);
  }

  function toggleAccount(id: string) {
    setAccountIds((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
    setRecommendation(null);
    setRevalidationPending(false);
  }

  async function chooseIpo(item: IpoCatalogItem, investable: boolean) {
    setCatalogueBusy(true);
    setCatalogueError("");
    try {
      const details = await bridge.invoke<IpoCatalogItem>("get_ipo_details", {
        sourceIpoId: item.source_ipo_id,
      });
      setSelectedDetails(details);
      if (investable && details.status === "OPEN") {
        const lots = details.minimum_lots ?? 1;
        const metadataSnapshot = metadataSnapshotFromDetails(details, lots, accountIds.length);
        updateIpo(ipos[0].id, {
          name: details.name,
          amountRupees: rupeesInputFromPaise(metadataSnapshot?.amount_per_account_paise ?? null),
          registrarId: registrarIdFromDetails(details),
          expectedAllotmentDate: details.allotment_date ?? "",
          lots: String(lots),
          metadataSnapshot,
        });
        if (!metadataSnapshot) {
          setCatalogueError(
            "This IPO is OPEN but its price or lot data is incomplete. Continue with manual entry.",
          );
        }
      } else if (investable) {
        setCatalogueError(
          "This IPO is no longer OPEN. Review the details or continue with manual entry.",
        );
      }
    } catch {
      setCatalogueError("IPO details are unavailable. You can continue with manual entry.");
    } finally {
      setCatalogueBusy(false);
    }
  }

  function validate(): string | null {
    const selectedIpos = ipos.filter((ipo) => ipo.included);
    if (selectedIpos.length === 0) return "Select at least one IPO.";
    if (selectedIpos.some((ipo) => ipo.metadataSnapshot && !officialPlanFor(ipo))) {
      return "Each official IPO needs a valid lot count before continuing.";
    }
    if (selectedIpos.every((ipo) => ipo.metadataSnapshot) && calculatedTotalCapital == null) {
      return "Calculated total capital is too large to submit safely.";
    }
    if (selectedIpos.some((ipo) => !ipo.name.trim() || amountPaiseFor(ipo) <= 0)) {
      return "Each selected IPO needs a name and positive amount per account.";
    }
    if (accountIds.length === 0) return "Select at least one member or friend account.";
    if (calculatedTotalCapital == null && paiseFromRupees(dailyRupees) <= 0) {
      return "Enter a positive daily investment.";
    }
    return null;
  }

  async function check() {
    const validation = validate();
    if (validation) {
      setError(validation);
      return;
    }
    setBusy(true);
    setError("");
    try {
      const response = await bridge.invoke<CheckResponse>("check_recommendation", {
        request: {
          session_id: sessionId.current,
          declared_capital_paise: calculatedTotalCapital ?? paiseFromRupees(dailyRupees),
          account_ids: accountIds,
          ipos: ipos
            .filter((ipo) => ipo.included)
            .map((ipo) => ({
              name: ipo.name.trim(),
              amount_paise: amountPaiseFor(ipo),
            })),
        },
      });
      setRecommendation(response);
    } catch (cause) {
      setError(userFacingError(cause, "The request could not be completed. Try again."));
    } finally {
      setBusy(false);
    }
  }

  function metadataSnapshotForSubmission(ipo: IpoDraft): IpoMetadataSnapshot | null {
    if (!ipo.metadataSnapshot) return null;
    const plan = officialPlanFor(ipo);
    if (!plan) return null;
    return {
      ...ipo.metadataSnapshot,
      lots: Number(ipo.lots),
      quantity: plan.quantity,
      amount_per_account_paise: plan.amountPerAccountPaise,
      total_capital_paise: plan.totalCapitalPaise,
    };
  }

  async function submit(confirmMetadataChanges = false) {
    if (!recommendation) return;
    setBusy(true);
    setError("");
    try {
      await bridge.invoke("submit_investment", {
        request: {
          session_id: sessionId.current,
          actor_member_id: members[0].id,
          declared_capital_paise: calculatedTotalCapital ?? paiseFromRupees(dailyRupees),
          recommendation_id: null,
          ipos: ipos
            .filter((ipo) => ipo.included)
            .map((ipo) => ({
              name: ipo.name.trim(),
              amount_paise: amountPaiseFor(ipo),
              account_ids: accountIds,
              registrar_id: ipo.registrarId,
              expected_allotment_date: ipo.expectedAllotmentDate || null,
              metadata_snapshot: metadataSnapshotForSubmission(ipo),
              confirm_metadata_changes: confirmMetadataChanges,
            })),
        },
      });
      setRevalidationPending(false);
      setMessage("Investment session submitted");
      await onSubmitted();
    } catch (cause) {
      const message = userFacingError(cause, "The request could not be completed. Try again.");
      setError(message);
      setRevalidationPending(/owner confirmation required/i.test(message));
    } finally {
      setBusy(false);
    }
  }

  if (historicalMode) {
    return (
      <HistoricalApplicationForm
        bridge={bridge}
        members={members}
        onCancel={() => setHistoricalMode(false)}
        onSubmitted={onSubmitted}
      />
    );
  }

  return (
    <div className="page-content invest-page">
      <section className="section-heading invest-heading">
        <div>
          <p className="eyebrow">Investment session</p>
          <h1>Plan first. Ask safely. Submit deliberately.</h1>
          <p>
            CHECK securely sends only approved non-secret session and account references, capital,
            IPO names, and amounts.
          </p>
          <button className="text-button" type="button" onClick={() => setHistoricalMode(true)}>
            Add historical application
          </button>
        </div>
        <label className="daily-investment">
          Daily investment (₹)
          <input
            aria-label="Daily investment (₹)"
            inputMode="decimal"
            placeholder="0"
            required
            value={dailyRupees}
            onChange={(e) => {
              setDailyRupees(e.target.value);
              setRecommendation(null);
            }}
          />
          <small className="daily-hint">Editable daily cap · persisted as integer paise</small>
        </label>
      </section>

      <section className="ipo-catalogue" aria-labelledby="available-ipos-heading">
        <div className="section-heading compact-heading">
          <div>
            <p className="eyebrow">Official Upstox metadata</p>
            <h2 id="available-ipos-heading">Available IPOs</h2>
          </div>
          {catalogueBusy && <span className="muted-label">Refreshing…</span>}
        </div>
        {catalogueError && <p className="inline-error">{catalogueError}</p>}
        {!catalogueError && (openCatalogue?.safe_message || upcomingCatalogue?.safe_message) && (
          <p className="inline-error">
            {openCatalogue?.safe_message ?? upcomingCatalogue?.safe_message}
          </p>
        )}
        {!catalogueError && !openCatalogue && !catalogueBusy && (
          <p className="empty-account-note">No catalogue data available. Use manual entry below.</p>
        )}
        <div className="ipo-catalogue-columns">
          <div>
            <h3>OPEN · investable</h3>
            {(openCatalogue?.items ?? []).map((item) => (
              <article className="ipo-catalogue-card" key={item.source_ipo_id}>
                <div>
                  <strong>{item.name}</strong>
                  <small>{item.symbol || item.issue_type}</small>
                </div>
                <p>
                  {formatOptionalRupees(item.minimum_price_paise)} –{" "}
                  {formatOptionalRupees(item.maximum_price_paise)} · {item.price_basis}
                </p>
                <button type="button" onClick={() => void chooseIpo(item, true)}>
                  View details & auto-fill
                </button>
              </article>
            ))}
            {openCatalogue && openCatalogue.items.length === 0 && (
              <p className="empty-account-note">No OPEN IPOs returned.</p>
            )}
          </div>
          <div>
            <h3>UPCOMING · view-only</h3>
            {(upcomingCatalogue?.items ?? []).map((item) => (
              <article className="ipo-catalogue-card" key={item.source_ipo_id}>
                <div>
                  <strong>{item.name}</strong>
                  <small>{item.symbol || item.issue_type}</small>
                </div>
                <p>
                  {formatOptionalRupees(item.minimum_price_paise)} –{" "}
                  {formatOptionalRupees(item.maximum_price_paise)} · {item.price_basis}
                </p>
                <button type="button" onClick={() => void chooseIpo(item, false)}>
                  View details
                </button>
              </article>
            ))}
            {upcomingCatalogue && upcomingCatalogue.items.length === 0 && (
              <p className="empty-account-note">No UPCOMING IPOs returned.</p>
            )}
          </div>
        </div>
        {selectedDetails && (
          <article className="ipo-details" aria-live="polite">
            <div className="section-heading compact-heading">
              <div>
                <p className="eyebrow">IPO details</p>
                <h3>{selectedDetails.name}</h3>
              </div>
              <span className="dev-label">{selectedDetails.status}</span>
            </div>
            {selectedDetails.safe_message && (
              <p className="inline-error">{selectedDetails.safe_message}</p>
            )}
            <dl className="ipo-detail-grid">
              <div>
                <dt>Price band</dt>
                <dd>
                  {formatOptionalRupees(selectedDetails.minimum_price_paise)} –{" "}
                  {formatOptionalRupees(selectedDetails.maximum_price_paise)}
                </dd>
              </div>
              <div>
                <dt>Cut-off / planning price</dt>
                <dd>
                  {formatOptionalRupees(selectedDetails.planning_price_paise)} ·{" "}
                  {selectedDetails.price_basis}
                </dd>
              </div>
              <div>
                <dt>Lot size</dt>
                <dd>{selectedDetails.lot_size ?? "TBA"}</dd>
              </div>
              <div>
                <dt>Minimum quantity</dt>
                <dd>{selectedDetails.minimum_quantity ?? "TBA"}</dd>
              </div>
              <div>
                <dt>Cost per lot</dt>
                <dd>{formatOptionalRupees(selectedDetails.cost_per_lot_paise)}</dd>
              </div>
              <div>
                <dt>Minimum application</dt>
                <dd>{formatOptionalRupees(selectedDetails.minimum_application_amount_paise)}</dd>
              </div>
              <div>
                <dt>Bidding dates</dt>
                <dd>
                  {selectedDetails.bidding_start_date ?? "TBA"} →{" "}
                  {selectedDetails.bidding_end_date ?? "TBA"}
                </dd>
              </div>
              <div>
                <dt>Allotment / listing</dt>
                <dd>
                  {selectedDetails.allotment_date ?? "TBA"} /{" "}
                  {selectedDetails.listing_date ?? "TBA"}
                </dd>
              </div>
              <div>
                <dt>Registrar</dt>
                <dd>
                  {selectedDetails.registrar_name ?? "TBA"}
                  {selectedDetails.registrar_mapping_state === "UNKNOWN"
                    ? " · unsupported for automation"
                    : ""}
                </dd>
              </div>
              <div>
                <dt>Subscription</dt>
                <dd>{selectedDetails.total_subscription ?? "TBA"}</dd>
              </div>
            </dl>
          </article>
        )}
      </section>

      <section className="studio-grid">
        <div className="form-panel investment-form">
          <fieldset className="ipo-fieldset">
            <legend>Applying for · manual entry remains available</legend>
            {ipos.map((ipo, index) => (
              <div className="ipo-row" key={ipo.id}>
                <label className="include-control">
                  <input
                    checked={ipo.included}
                    onChange={(e) => updateIpo(ipo.id, { included: e.target.checked })}
                    type="checkbox"
                  />
                  <span>{String(index + 1).padStart(2, "0")}</span>
                </label>
                <label>
                  IPO name
                  <input
                    disabled={Boolean(ipo.metadataSnapshot)}
                    required={ipo.included}
                    value={ipo.name}
                    onChange={(e) => updateIpo(ipo.id, { name: e.target.value })}
                  />
                </label>
                <label>
                  Registrar
                  <select
                    aria-label="Registrar"
                    disabled={Boolean(ipo.metadataSnapshot)}
                    value={ipo.registrarId}
                    onChange={(e) => updateIpo(ipo.id, { registrarId: e.target.value })}
                  >
                    <option value="kfintech">KFintech</option>
                    <option value="bigshare">Bigshare Services</option>
                    <option value="mufg_intime">MUFG Intime India</option>
                  </select>
                </label>
                <label>
                  Expected allotment date
                  <input
                    aria-label="Expected allotment date"
                    disabled={Boolean(ipo.metadataSnapshot)}
                    type="date"
                    value={ipo.expectedAllotmentDate}
                    onChange={(e) => updateIpo(ipo.id, { expectedAllotmentDate: e.target.value })}
                  />
                </label>
                {ipo.metadataSnapshot && (
                  <label>
                    Lots
                    <input
                      aria-label="Lots"
                      min={ipo.metadataSnapshot.minimum_lots ?? 1}
                      step="1"
                      type="number"
                      value={ipo.lots}
                      onChange={(e) => updateIpo(ipo.id, { lots: e.target.value })}
                    />
                  </label>
                )}
                <label>
                  Amount per account (₹)
                  <input
                    inputMode="decimal"
                    readOnly={Boolean(ipo.metadataSnapshot)}
                    required={ipo.included}
                    value={rupeesInputFromPaise(
                      officialPlanFor(ipo)?.amountPerAccountPaise ??
                        paiseFromRupees(ipo.amountRupees),
                    )}
                    onChange={(e) => updateIpo(ipo.id, { amountRupees: e.target.value })}
                  />
                </label>
                {ipo.metadataSnapshot && (
                  <div className="ipo-calculation" aria-live="polite">
                    <span>Quantity: {officialPlanFor(ipo)?.quantity ?? "TBA"}</span>
                    <span>Source: Upstox · {ipo.metadataSnapshot.source_ipo_id}</span>
                    <button
                      className="text-button"
                      onClick={() => updateIpo(ipo.id, { lots: "", metadataSnapshot: null })}
                      type="button"
                    >
                      Use manual entry
                    </button>
                  </div>
                )}
                {ipos.length > 1 && (
                  <button
                    aria-label={`Remove IPO ${index + 1}`}
                    className="remove-button"
                    onClick={() =>
                      setIpos((current) => current.filter((item) => item.id !== ipo.id))
                    }
                    type="button"
                  >
                    ×
                  </button>
                )}
              </div>
            ))}
            <button
              className="text-button add-row"
              onClick={() =>
                setIpos((current) => [
                  ...current,
                  {
                    id: newId("ipo"),
                    name: "",
                    amountRupees: "",
                    registrarId: "kfintech",
                    expectedAllotmentDate: "",
                    included: true,
                    lots: "",
                    metadataSnapshot: null,
                  },
                ])
              }
              type="button"
            >
              + Add another IPO
            </button>
            {calculatedTotalCapital != null && (
              <p className="calculated-capital" role="status">
                Total capital: {formatRupees(calculatedTotalCapital)}
              </p>
            )}
          </fieldset>

          <fieldset className="account-fieldset">
            <legend>Accounts to apply from</legend>
            <div className="account-options">
              {accounts.map((account) => (
                <label key={account.id}>
                  <input
                    aria-label={`${account.label} · ${account.pan}`}
                    checked={accountIds.includes(account.id)}
                    onChange={() => toggleAccount(account.id)}
                    type="checkbox"
                  />
                  <span>
                    <strong>{account.label}</strong>
                    <code>{account.pan}</code>
                  </span>
                </label>
              ))}
              {accounts.length === 0 && (
                <p className="empty-account-note">
                  Add a core member or friend account before checking this session.
                </p>
              )}
            </div>
          </fieldset>
          {error && (
            <div>
              <p className="inline-error" role="alert">
                {error}
              </p>
              {revalidationPending && (
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() => void submit(true)}
                  type="button"
                >
                  Confirm changes and SUBMIT
                </button>
              )}
            </div>
          )}
        </div>

        <aside className="recommendation-panel" aria-live="polite">
          <p className="eyebrow">Recommendation review</p>
          {!recommendation ? (
            <div className="recommendation-empty">
              <span>Preview only</span>
              <h2>Nothing leaves the private boundary yet.</h2>
              <p>
                CHECK securely sends only approved fields and runs the deterministic recommendation
                preview.
              </p>
            </div>
          ) : (
            <div className="recommendation-result">
              <div className="dev-label">{recommendation.label}</div>
              <p className="algorithm-version">{recommendation.algorithm_version}</p>
              <p>{recommendation.explanation}</p>
              <ol>
                {recommendation.ipos.map((ipo) => (
                  <li className={ipo.skip ? "skip" : ""} key={ipo.typed_name}>
                    <span className="rank">#{ipo.ranking}</span>
                    <div>
                      <strong>{ipo.typed_name}</strong>
                      <p>{ipo.reason}</p>
                      <small>
                        Score {ipo.score} · {ipo.recommended_account_count} accounts ·{" "}
                        {ipo.recommended_allocation_ratio_bp / 100}% allocation
                      </small>
                      <FinanceBar
                        label="Recommended allocation"
                        valueLabel={`${ipo.recommended_allocation_ratio_bp / 100}%`}
                        ratio={ipo.recommended_allocation_ratio_bp / 10000}
                        tone={ipo.skip ? "muted" : "accent"}
                      />
                      {ipo.missing_public_data.length > 0 && (
                        <small>Missing: {ipo.missing_public_data.join(", ")}</small>
                      )}
                    </div>
                  </li>
                ))}
              </ol>
            </div>
          )}
          {message && (
            <p className="success-message" role="status">
              {message}
            </p>
          )}
        </aside>
      </section>

      <fieldset className="action-rail" aria-label="Investment actions">
        <div>
          <span className={recommendation ? "rail-step complete" : "rail-step active"}>
            01 CHECK
          </span>
          <span className={recommendation ? "rail-step active" : "rail-step"}>02 EDIT</span>
          <span className="rail-step">03 SUBMIT</span>
        </div>
        <div className="action-buttons">
          <button
            className={recommendation ? "secondary-button" : "primary-button"}
            disabled={busy}
            onClick={() => void check()}
            type="button"
          >
            {busy && !recommendation ? "CHECKING…" : "CHECK"}
          </button>
          <button
            className="secondary-button edit-button"
            disabled={!recommendation || busy}
            onClick={() => setRecommendation(null)}
            type="button"
          >
            EDIT
          </button>
          <button
            className="primary-button submit-button"
            disabled={!recommendation || busy}
            onClick={() => void submit()}
            type="button"
          >
            SUBMIT
          </button>
        </div>
      </fieldset>
    </div>
  );
}

function ProfitEditor({
  bridge,
  report,
  row,
  actorMemberId,
}: {
  bridge: CommandBridge;
  report: AllotmentJobReport;
  row: AllotmentReportRow;
  actorMemberId: string;
}) {
  const [basis, setBasis] = useState("UNAVAILABLE");
  const [issuePrice, setIssuePrice] = useState("");
  const [referencePrice, setReferencePrice] = useState("");
  const [source, setSource] = useState("");
  const [asOf, setAsOf] = useState("");
  const [result, setResult] = useState<EstimatedProfit | null>(null);
  const [error, setError] = useState("");

  async function calculate() {
    setError("");
    try {
      const estimate = await bridge.invoke<EstimatedProfit>("estimate_profit", {
        request: {
          application_id: report.application_id,
          account_id: row.account_id,
          actor_member_id: actorMemberId,
          basis,
          allotted_shares: row.allotted_shares ?? 0,
          reference_price_paise: referencePrice ? paiseFromRupees(referencePrice) : null,
          issue_price_paise: issuePrice ? paiseFromRupees(issuePrice) : null,
          source,
          as_of: asOf,
          note: null,
        },
      });
      setResult(estimate);
    } catch (cause) {
      setError(userFacingError(cause, "The profit estimate could not be calculated. Try again."));
    }
  }

  return (
    <details className="profit-estimator">
      <summary>Estimated profit basis</summary>
      <p className="form-note">Estimate only. Realized profit is recorded separately.</p>
      <div className="compact-grid">
        <label>
          Basis
          <select
            aria-label={`Profit basis for ${row.display_name}`}
            value={basis}
            onChange={(e) => setBasis(e.target.value)}
          >
            <option value="UNAVAILABLE">Unavailable</option>
            <option value="ACTUAL_LISTING_PRICE">Actual listing price</option>
            <option value="CURRENT_MARKET_PRICE">Current market price</option>
            <option value="OWNER_EXPECTED_PRICE">Owner expected price</option>
            <option value="PUBLIC_ESTIMATE">Public estimate</option>
          </select>
        </label>
        <label>
          Issue price (₹)
          <input
            inputMode="decimal"
            value={issuePrice}
            onChange={(e) => setIssuePrice(e.target.value)}
          />
        </label>
        <label>
          Reference price (₹)
          <input
            inputMode="decimal"
            value={referencePrice}
            onChange={(e) => setReferencePrice(e.target.value)}
          />
        </label>
        <label>
          Source / owner
          <input value={source} onChange={(e) => setSource(e.target.value)} />
        </label>
        <label>
          As of
          <input
            placeholder="YYYY-MM-DD or market close"
            value={asOf}
            onChange={(e) => setAsOf(e.target.value)}
          />
        </label>
      </div>
      <button className="secondary-button" onClick={() => void calculate()} type="button">
        Save estimate basis
      </button>
      {result && (
        <p className="inline-ok" role="status">
          {result.estimated_profit_paise == null
            ? "Estimate unavailable: no price was invented."
            : `${formatRupees(result.estimated_profit_paise)} estimated · ${result.basis}`}
        </p>
      )}
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
    </details>
  );
}

function AllotmentView({ bridge, members }: { bridge: CommandBridge; members: MemberRow[] }) {
  const [candidates, setCandidates] = useState<AllotmentCandidate[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [report, setReport] = useState<AllotmentJobReport | null>(null);
  const [security, setSecurity] = useState<SecurityStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [lookupAuthorization, setLookupAuthorization] = useState<LookupAuthorizationStatus | null>(
    null,
  );
  const [lookupConfirmed, setLookupConfirmed] = useState(false);
  const [lookupBusy, setLookupBusy] = useState(false);
  const [manualAccountId, setManualAccountId] = useState("");
  const [manualResult, setManualResult] = useState("NOT_ALLOTTED");
  const [manualLots, setManualLots] = useState("");
  const [manualShares, setManualShares] = useState("");
  const [manualNote, setManualNote] = useState("");

  useEffect(() => {
    bridge
      .invoke<SecurityStatus>("get_security_status")
      .then(setSecurity)
      .catch((e) =>
        setError(
          userFacingError(
            e,
            "Security status is unavailable. Reopen the desktop app and try again.",
          ),
        ),
      );
    bridge
      .invoke<AllotmentCandidate[]>("list_allotment_candidates")
      .then((rows) => {
        setCandidates(rows);
        if (rows[0]) setSelected(rows[0].application_id);
      })
      .catch((e) =>
        setError(
          userFacingError(
            e,
            "Submitted applications are unavailable. Reopen the desktop app and try again.",
          ),
        ),
      );
  }, [bridge]);

  useEffect(() => {
    const candidate = candidates.find((item) => item.application_id === selected);
    setLookupAuthorization(null);
    setLookupConfirmed(false);
    if (!candidate) return;
    let active = true;
    bridge
      .invoke<LookupAuthorizationStatus>("get_lookup_authorization_status", {
        application_id: candidate.application_id,
      })
      .then((status) => {
        if (active) setLookupAuthorization(status);
      })
      .catch(() => {
        if (active) {
          setLookupAuthorization({
            provider_id: candidate.provider_id,
            status: "NOT_GRANTED",
          });
        }
      });
    return () => {
      active = false;
    };
  }, [bridge, candidates, selected]);

  useEffect(() => {
    if (
      !report ||
      ["COMPLETE", "COMPLETE_WITH_UNCONFIRMED", "NEEDS_HUMAN_VERIFICATION", "CANCELLED"].includes(
        report.status,
      )
    ) {
      setBusy(false);
      return;
    }
    const timer = window.setInterval(() => {
      bridge
        .invoke<AllotmentJobReport>("get_allotment_report", { jobId: report.job_id })
        .then((next) => {
          setReport(next);
          setMessage(`Job ${next.job_id} → ${next.status}`);
        })
        .catch((cause) =>
          setError(userFacingError(cause, "The allotment job could not be refreshed. Try again.")),
        );
    }, 750);
    return () => window.clearInterval(timer);
  }, [bridge, report]);

  useEffect(() => {
    if (report?.accounts[0] && !manualAccountId) {
      setManualAccountId(report.accounts[0].account_id);
    }
  }, [manualAccountId, report]);

  async function runCheck() {
    const candidate = candidates.find((c) => c.application_id === selected);
    if (!candidate) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const result = await bridge.invoke<AllotmentJobReport>("start_allotment_check", {
        request: {
          application_id: candidate.application_id,
          session_id: candidate.session_id,
          ipo_name: candidate.ipo_name,
          actor_member_id: members[0]?.id ?? "unknown",
          registrar_id: candidate.registrar_id,
        },
      });
      setReport(result);
      setMessage(`Job ${result.job_id} → ${result.status}`);
    } catch (e) {
      setError(
        userFacingError(
          e,
          "The allotment check is unavailable. Reopen the desktop app and try again.",
        ),
      );
      setBusy(false);
    }
  }

  async function authorizeLookup() {
    const candidate = candidates.find((c) => c.application_id === selected);
    const owner = members.find((member) => member.role === "OWNER");
    if (!candidate || !owner || !lookupConfirmed) return;
    setLookupBusy(true);
    setError(null);
    setMessage(null);
    try {
      const next = await bridge.invoke<LookupAuthorizationStatus>(
        "authorize_real_investor_lookup",
        {
          request: {
            application_id: candidate.application_id,
            actor_member_id: owner.id,
            owner_affirmed: true,
          },
        },
      );
      setLookupAuthorization(next);
      setLookupConfirmed(false);
      setMessage("One real investor lookup authorized through the owner-controlled gate.");
    } catch (cause) {
      setError(userFacingError(cause, "The real investor lookup could not be authorized."));
    } finally {
      setLookupBusy(false);
    }
  }

  async function saveManualResult(event: FormEvent) {
    event.preventDefault();
    if (!report || !manualAccountId) return;
    setError(null);
    try {
      await bridge.invoke("record_manual_allotment", {
        request: {
          job_id: report.job_id,
          account_id: manualAccountId,
          actor_member_id: members[0]?.id ?? "unknown",
          allotted_lots: manualLots ? Number(manualLots) : null,
          allotted_shares: manualShares ? Number(manualShares) : null,
          explicit_not_allotted: manualResult === "NOT_ALLOTTED",
          note: manualNote || null,
        },
      });
      const next = await bridge.invoke<AllotmentJobReport>("get_allotment_report", {
        jobId: report.job_id,
      });
      setReport(next);
      setMessage("Manual result saved with explicit MANUAL provenance.");
    } catch (cause) {
      setError(userFacingError(cause, "The manual result could not be saved. Try again."));
    }
  }

  const selectedCandidate = candidates.find((candidate) => candidate.application_id === selected);
  const owner = members.find((member) => member.role === "OWNER");
  const securityReady = security?.mode === "PRODUCTION_SECURE" && security.real_pan_allowed;
  const keyringReady = security?.key_provider === "os-keyring" && security.real_pan_allowed;

  return (
    <div className="page-content">
      {owner && (
        <section aria-label="Real Investor Lookup" className="panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Owner control</p>
              <h2>Real Investor Lookup</h2>
              <p>
                A separate, one-shot authorization is required before any production PAN lookup. It
                expires after five minutes and is never stored in SQLite.
              </p>
            </div>
          </div>
          <dl className="ipo-detail-grid">
            <div>
              <dt>Security</dt>
              <dd>{securityReady ? "PASS" : "FAIL"}</dd>
            </div>
            <div>
              <dt>Keyring</dt>
              <dd>{keyringReady ? "PASS" : "FAIL"}</dd>
            </div>
            <div>
              <dt>Lookup Authorization</dt>
              <dd>{lookupAuthorization?.status ?? "NOT_GRANTED"}</dd>
            </div>
          </dl>
          <label className="choice-row">
            <input
              checked={lookupConfirmed}
              onChange={(event) => setLookupConfirmed(event.target.checked)}
              type="checkbox"
            />
            <span>
              <strong>I explicitly authorize one real investor lookup</strong>
              <small>The selected application and its resolved registrar are the scope.</small>
            </span>
          </label>
          <button
            className="secondary-button"
            disabled={
              !lookupConfirmed ||
              lookupBusy ||
              !selectedCandidate ||
              !securityReady ||
              lookupAuthorization?.status === "ACTIVE"
            }
            onClick={() => void authorizeLookup()}
            type="button"
          >
            {lookupBusy ? "Authorizing…" : "Authorize One Lookup"}
          </button>
        </section>
      )}
      <section className="panel">
        <div className="panel-heading">
          <div>
            <p className="eyebrow">Registrar allotment</p>
            <h1>Check Allotment</h1>
            <p>
              Durable local jobs check each account sequentially. PAN decrypts only inside the
              purpose-scoped provider call and is never stored in job state.
            </p>
          </div>
        </div>
        {security && (
          <div
            className={
              security.real_pan_allowed ? "security-strip ready" : "security-strip blocker"
            }
            role="status"
          >
            <strong>{security.mode}</strong>
            <span>
              {security.real_pan_allowed
                ? `OS keyring verified · ${security.key_provider}`
                : (security.blocker ?? "Real PAN is blocked")}
            </span>
          </div>
        )}
        {error && (
          <p className="inline-error" role="alert">
            {error}
          </p>
        )}
        {message && (
          <p className="inline-ok" role="status">
            {message}
          </p>
        )}
        <div className="stack-list">
          {candidates.length === 0 && (
            <p>No submitted IPO applications yet. Submit an investment first.</p>
          )}
          {candidates.map((c) => (
            <label className="choice-row" key={c.application_id}>
              <input
                checked={selected === c.application_id}
                name="allotment-ipo"
                onChange={() => setSelected(c.application_id)}
                type="radio"
              />
              <span>
                <strong>{c.ipo_name}</strong>
                <span>{c.registrar_name}</span>
                <small>
                  {c.account_count} account
                  {c.account_count === 1 ? "" : "s"} · {formatRupees(c.planned_amount_paise)}{" "}
                  planned · {c.overall_job_state.replaceAll("_", " ").toLowerCase()}
                </small>
                <StatusBadge status={c.overall_job_state} />
                {c.provider_health === "HUMAN_VERIFICATION_REQUIRED" && (
                  <StatusBadge status="NEEDS_HUMAN_VERIFICATION" />
                )}
                {c.provider_id === "unsupported" && (
                  <StatusBadge status="MANUAL_FALLBACK_REQUIRED" />
                )}
                <ProgressMeter
                  label={`${c.ipo_name} accounts finalized`}
                  done={c.final_count}
                  total={
                    c.final_count + c.pending_count > 0
                      ? c.final_count + c.pending_count
                      : c.account_count
                  }
                />
              </span>
            </label>
          ))}
        </div>
        <footer className="action-rail">
          <button
            className="primary-button"
            disabled={
              !selected ||
              busy ||
              candidates.find((candidate) => candidate.application_id === selected)?.provider_id ===
                "unsupported"
            }
            onClick={runCheck}
            type="button"
          >
            {busy
              ? "Queued · running in background…"
              : candidates.length
                ? "Check All Accounts"
                : "No applications to check"}
          </button>
          {busy && report && (
            <button
              className="secondary-button"
              onClick={() => void bridge.invoke("cancel_allotment_job", { jobId: report.job_id })}
              type="button"
            >
              Cancel safely
            </button>
          )}
        </footer>
      </section>
      {report && (
        <section aria-label="Allotment report card" className="panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Report card</p>
              <h2>{report.ipo_name}</h2>
              <p>Registrar: {report.registrar_name}</p>
              <p className="report-status">
                <StatusBadge status={report.status} />
                <span>
                  {report.status === "PARTIALLY_COMPLETE"
                    ? `Partially complete · ${report.final_count} of ${report.accounts.length} final`
                    : `${report.final_count} of ${report.accounts.length} final`}
                </span>
              </p>
            </div>
          </div>
          <div className="report-summary finance-stack">
            <ProgressMeter
              label="Accounts finalized"
              done={report.final_count}
              total={report.accounts.length}
            />
            <FinanceBar
              label="Pending accounts"
              valueLabel={`${report.pending_count}`}
              ratio={
                report.accounts.length === 0 ? 0 : report.pending_count / report.accounts.length
              }
              tone="warning"
            />
            <FinanceBar
              label="Applied capital (sum)"
              valueLabel={formatRupees(
                report.accounts.reduce((sum, row) => sum + row.application_amount_paise, 0),
              )}
              ratio={1}
              tone="accent"
            />
          </div>
          <ul className="stack-list">
            {report.accounts.map((row) => {
              const amountMax = Math.max(
                ...report.accounts.map((entry) => entry.application_amount_paise),
                1,
              );
              return (
                <li key={row.attempt_id}>
                  <strong>
                    {row.display_name} · {row.account_kind}
                  </strong>
                  <StatusBadge status={row.status} />
                  <small>
                    {row.masked_pan} · {sourceLabel(row.source)} · {provenanceLabel(row.provenance)}
                    {row.allotted_lots != null ? ` · ${row.allotted_lots} lot(s)` : ""}
                    {row.allotted_shares != null ? ` / ${row.allotted_shares} shares` : ""}
                    {` · ${formatRupees(row.application_amount_paise)} applied`}
                  </small>
                  <FinanceBar
                    label="Applied amount"
                    valueLabel={formatRupees(row.application_amount_paise)}
                    ratio={row.application_amount_paise / amountMax}
                    tone="accent"
                  />
                  {row.estimated_profit_paise != null && (
                    <FinanceBar
                      label="Estimated profit"
                      valueLabel={formatRupees(row.estimated_profit_paise)}
                      ratio={
                        row.application_amount_paise === 0
                          ? 0
                          : Math.min(
                              1,
                              Math.abs(row.estimated_profit_paise) / row.application_amount_paise,
                            )
                      }
                      tone="positive"
                    />
                  )}
                  {row.safe_message && <p>{row.safe_message}</p>}
                  {row.human_verification_state && report.official_status_url && (
                    <button
                      aria-label={`Continue Verification for ${row.display_name}`}
                      className="secondary-button"
                      onClick={() =>
                        window.open(
                          report.official_status_url ?? "",
                          "_blank",
                          "noopener,noreferrer",
                        )
                      }
                      type="button"
                    >
                      Continue Verification
                    </button>
                  )}
                  {row.estimated_profit_paise != null && (
                    <p>
                      Estimated profit {formatRupees(row.estimated_profit_paise)} ·{" "}
                      {row.profit_basis}
                      {row.profit_provenance ? ` · ${row.profit_provenance}` : ""}
                    </p>
                  )}
                  {row.allotted_shares != null && row.allotted_shares > 0 && (
                    <ProfitEditor
                      actorMemberId={members[0]?.id ?? "unknown"}
                      bridge={bridge}
                      report={report}
                      row={row}
                    />
                  )}
                </li>
              );
            })}
          </ul>
          {report.official_status_url && (
            <p>
              <a href={report.official_status_url} rel="noreferrer" target="_blank">
                Open Official Page
              </a>{" "}
              if CAPTCHA, OTP, or browser verification is required. Return here to save the result
              manually.
            </p>
          )}
          {report.accounts.length > 0 && (
            <form className="manual-result-form" onSubmit={saveManualResult}>
              <h3>Record manual result</h3>
              <p className="form-note">
                Saved as MANUAL with local actor, device, and event provenance.
              </p>
              <div className="compact-grid">
                <label>
                  Account
                  <select
                    value={manualAccountId}
                    onChange={(e) => setManualAccountId(e.target.value)}
                  >
                    {report.accounts.map((row) => (
                      <option key={row.account_id} value={row.account_id}>
                        {row.display_name} · {row.masked_pan}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Result
                  <select value={manualResult} onChange={(e) => setManualResult(e.target.value)}>
                    <option value="NOT_ALLOTTED">Report not allotted</option>
                    <option value="ALLOTTED">Report allotted</option>
                    <option value="UNCONFIRMED">Unconfirmed</option>
                  </select>
                </label>
                <label>
                  Lots
                  <input
                    inputMode="numeric"
                    value={manualLots}
                    onChange={(e) => setManualLots(e.target.value)}
                  />
                </label>
                <label>
                  Shares
                  <input
                    inputMode="numeric"
                    value={manualShares}
                    onChange={(e) => setManualShares(e.target.value)}
                  />
                </label>
                <label>
                  Note
                  <input value={manualNote} onChange={(e) => setManualNote(e.target.value)} />
                </label>
              </div>
              <button className="secondary-button" type="submit">
                Save manual provenance
              </button>
            </form>
          )}
        </section>
      )}
    </div>
  );
}

export function App({ bridge = defaultBridge }: { bridge?: CommandBridge }) {
  const [boot, setBoot] = useState<BootState>("loading");
  const [view, setView] = useState<View>("dashboard");
  const [members, setMembers] = useState<MemberRow[]>([]);
  const [friends, setFriends] = useState<FriendRow[]>([]);
  const [dashboard, setDashboard] = useState<DashboardData>(EMPTY_DASHBOARD);

  const loadDashboard = useCallback(async () => {
    const next = await bridge.invoke<DashboardData>("get_dashboard");
    setDashboard(next);
  }, [bridge]);

  useEffect(() => {
    let active = true;
    Promise.all([
      bridge.invoke<MemberRow[]>("list_members"),
      bridge.invoke<FriendRow[]>("list_friends"),
      bridge.invoke<DashboardData>("get_dashboard"),
    ])
      .then(([nextMembers, nextFriends, nextDashboard]) => {
        if (!active) return;
        setMembers(nextMembers);
        setFriends(nextFriends);
        setDashboard(nextDashboard);
        setBoot("ready");
      })
      .catch(() => {
        if (active) setBoot("offline");
      });
    return () => {
      active = false;
    };
  }, [bridge]);

  if (boot === "ready" && members.length === 0) {
    return (
      <Onboarding
        bridge={bridge}
        onCreated={(member) => {
          setMembers([member]);
          setDashboard((current) => ({ ...current, member_count: 1 }));
          setView("dashboard");
        }}
      />
    );
  }

  return (
    <AppShell active={view} memberCount={members.length + friends.length} setActive={setView}>
      {boot === "loading" && (
        <div className="loading-bar" aria-label="Loading local vault" role="progressbar" />
      )}
      {view === "dashboard" && (
        <DashboardView
          bridge={bridge}
          dashboard={dashboard}
          ownerMemberId={members.find((member) => member.role === "OWNER")?.id}
          onAllotment={() => setView("allotment")}
          onChanged={loadDashboard}
          onInvest={() => setView("invest")}
        />
      )}
      {view === "members" && (
        <MembersView
          bridge={bridge}
          friends={friends}
          members={members}
          onFriendAdded={(friend) => {
            setFriends((current) => [...current, friend]);
            setDashboard((current) => ({ ...current, friend_count: current.friend_count + 1 }));
          }}
          onFriendArchived={(id) => {
            setFriends((current) => current.filter((friend) => friend.id !== id));
            setDashboard((current) => ({
              ...current,
              friend_count: Math.max(0, current.friend_count - 1),
            }));
          }}
        />
      )}
      {view === "invest" && (
        <InvestView
          bridge={bridge}
          friends={friends}
          members={members}
          onSubmitted={loadDashboard}
        />
      )}
      {view === "allotment" && <AllotmentView bridge={bridge} members={members} />}
      {view === "settings" && <SettingsView bridge={bridge} />}
    </AppShell>
  );
}
