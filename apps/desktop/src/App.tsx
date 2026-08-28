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
  included: boolean;
}

type View = "dashboard" | "members" | "invest" | "allotment";
type BootState = "loading" | "ready" | "offline";

interface AllotmentCandidate {
  application_id: string;
  session_id: string;
  ipo_name: string;
  planned_amount_paise: number;
  account_count: number;
  registrar_id: string;
  registrar_name: string;
  official_status_url?: string | null;
  provider_status: string;
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
  source: string;
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
  accounts: AllotmentReportRow[];
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

function Mark() {
  return (
    <span className="brand-mark" aria-hidden="true">
      S
    </span>
  );
}

function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 24 24">
      <path d="m5 12 14 0M13 6l6 6-6 6" />
    </svg>
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
  const items: Array<{ label: string; view: View }> = [
    { label: "Dashboard", view: "dashboard" },
    { label: "Members", view: "members" },
    { label: "Investments", view: "invest" },
    { label: "Allotment", view: "allotment" },
  ];

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-row">
          <Mark />
          <div>
            <strong>Sanket IPO</strong>
            <span>Private group OS</span>
          </div>
        </div>
        <nav aria-label="Primary navigation">
          <p className="eyebrow">Private workspace</p>
          <ul>
            {items.map((item) => (
              <li key={item.label}>
                <button
                  className={active === item.view ? "nav-item active" : "nav-item"}
                  onClick={() => setActive(item.view)}
                  type="button"
                >
                  <span className="nav-index" aria-hidden="true">
                    0{items.indexOf(item) + 1}
                  </span>
                  {item.label}
                </button>
              </li>
            ))}
          </ul>
        </nav>
        <div className="privacy-card">
          <span className="privacy-indicator" aria-hidden="true" />
          <div>
            <strong>Private domain locked</strong>
            <span>PAN and UPI stay in the encrypted local vault.</span>
          </div>
        </div>
        <div className="profile-button">
          <span className="avatar" aria-hidden="true">
            {memberCount || "—"}
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
      </aside>
      <main>
        <header className="topbar">
          <div>
            <span className="saved-status">
              <span aria-hidden="true" /> Saved locally
            </span>
            <span className="sync-status">Pending sync</span>
          </div>
          <span className="topbar-seal">PRIVATE · LOCAL · AUDITED</span>
        </header>
        {children}
      </main>
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
      setError(cause instanceof Error ? cause.message : String(cause));
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
        {error && <p className="inline-error">{error}</p>}
        <button className="primary-button" disabled={busy} type="submit">
          {busy ? "Creating encrypted profile…" : "Create private profile"}
        </button>
      </form>
    </main>
  );
}

function DashboardView({
  dashboard,
  onInvest,
  onAllotment,
}: {
  dashboard: DashboardData;
  onInvest: () => void;
  onAllotment: () => void;
}) {
  const metrics = [
    {
      label: "Group investment",
      value: formatRupees(dashboard.total_planned_paise),
      detail: `${dashboard.submitted_session_count} submitted session${dashboard.submitted_session_count === 1 ? "" : "s"}`,
    },
    {
      label: "Group net profit",
      value: formatRupees(dashboard.profit_paise),
      detail: "Available after allotment records",
    },
    {
      label: "Core members",
      value: String(dashboard.member_count),
      detail: "Encrypted identity profiles",
    },
    {
      label: "Active friend accounts",
      value: String(dashboard.friend_count),
      detail: "Archive instead of delete",
    },
  ];

  return (
    <div className="dashboard page-content">
      <section className="hero" aria-labelledby="dashboard-heading">
        <div>
          <p className="eyebrow">Group overview · projection-backed</p>
          <h1 id="dashboard-heading">Every rupee, account, and decision in one private ledger.</h1>
          <p className="hero-copy">
            Plan locally, review a clearly labeled development recommendation, then submit the human
            decision as immutable events.
          </p>
        </div>
        <div className="quick-actions" aria-label="Quick actions">
          <button aria-label="Invest" className="primary-action" onClick={onInvest} type="button">
            <span>Invest</span>
            <small>CHECK, edit, then submit</small>
            <ArrowIcon />
          </button>
          <button
            aria-label="Check Allotment"
            className="secondary-action"
            onClick={onAllotment}
            type="button"
          >
            <span>Check Allotment</span>
            <small>Fixture registrar · purpose-scoped PAN</small>
            <ArrowIcon />
          </button>
        </div>
      </section>
      <section className="metric-grid" aria-label="Group metrics">
        {metrics.map((metric) => (
          <article className="metric-card" key={metric.label}>
            <p>{metric.label}</p>
            <strong>{metric.value}</strong>
            <span>{metric.detail}</span>
          </article>
        ))}
      </section>
      <section className="content-grid">
        <article className="panel capital-rail-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Decision rail</p>
              <h2>Local investment workflow</h2>
            </div>
          </div>
          <ol className="decision-rail">
            <li>
              <span>01</span>
              <strong>CHECK</strong>
              <p>Build the approved multi-IPO request. No private identity fields.</p>
            </li>
            <li>
              <span>02</span>
              <strong>EDIT</strong>
              <p>Keep the human in control of accounts and planned amounts.</p>
            </li>
            <li>
              <span>03</span>
              <strong>SUBMIT</strong>
              <p>Persist events and rebuildable local projections.</p>
            </li>
          </ol>
        </article>
        <article className="panel activity-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Status</p>
              <h2>Private foundation</h2>
            </div>
          </div>
          <div className="activity-empty">
            <span className="activity-glyph" aria-hidden="true">
              ✓
            </span>
            <strong>Local-first core ready</strong>
            <p>Submitted investment sessions now feed the dashboard from SQLite projections.</p>
          </div>
        </article>
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
      setError(cause instanceof Error ? cause.message : String(cause));
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
          className="primary-button"
          onClick={() => setAdding((value) => !value)}
          type="button"
        >
          {adding ? "Close form" : "Add friend account"}
        </button>
      </section>

      {adding && (
        <form className="form-panel friend-form" onSubmit={submit}>
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
          {error && <p className="inline-error">{error}</p>}
          <button className="primary-button" type="submit">
            Encrypt and add friend
          </button>
        </form>
      )}

      <section className="account-list" aria-label="Core members">
        <p className="eyebrow">Core members</p>
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
            No friend accounts yet. Add one without exposing plaintext PAN.
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
  const [dailyRupees, setDailyRupees] = useState("");
  const [ipos, setIpos] = useState<IpoDraft[]>([
    { id: newId("ipo"), name: "", amountRupees: "", included: true },
  ]);
  const [accountIds, setAccountIds] = useState<string[]>([]);
  const [recommendation, setRecommendation] = useState<CheckResponse | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const accounts = useMemo(
    () => [
      ...members.map((member) => ({ id: member.id, label: member.name, pan: member.masked_pan })),
      ...friends.map((friend) => ({ id: friend.id, label: friend.label, pan: friend.masked_pan })),
    ],
    [members, friends],
  );

  function updateIpo(id: string, patch: Partial<IpoDraft>) {
    setIpos((current) => current.map((ipo) => (ipo.id === id ? { ...ipo, ...patch } : ipo)));
    setRecommendation(null);
    setMessage("");
  }

  function toggleAccount(id: string) {
    setAccountIds((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
    setRecommendation(null);
  }

  function validate(): string | null {
    if (paiseFromRupees(dailyRupees) <= 0) return "Enter a positive daily investment.";
    const selectedIpos = ipos.filter((ipo) => ipo.included);
    if (selectedIpos.length === 0) return "Select at least one IPO.";
    if (selectedIpos.some((ipo) => !ipo.name.trim() || paiseFromRupees(ipo.amountRupees) <= 0)) {
      return "Each selected IPO needs a name and positive amount per account.";
    }
    if (accountIds.length === 0) return "Select at least one member or friend account.";
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
          declared_capital_paise: paiseFromRupees(dailyRupees),
          account_ids: accountIds,
          ipos: ipos
            .filter((ipo) => ipo.included)
            .map((ipo) => ({
              name: ipo.name.trim(),
              amount_paise: paiseFromRupees(ipo.amountRupees),
            })),
        },
      });
      setRecommendation(response);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  async function submit() {
    if (!recommendation) return;
    setBusy(true);
    setError("");
    try {
      await bridge.invoke("submit_investment", {
        request: {
          session_id: sessionId.current,
          actor_member_id: members[0].id,
          declared_capital_paise: paiseFromRupees(dailyRupees),
          recommendation_id: null,
          ipos: ipos
            .filter((ipo) => ipo.included)
            .map((ipo) => ({
              name: ipo.name.trim(),
              amount_paise: paiseFromRupees(ipo.amountRupees),
              account_ids: accountIds,
            })),
        },
      });
      setMessage("Investment session submitted");
      await onSubmitted();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page-content invest-page">
      <section className="section-heading invest-heading">
        <div>
          <p className="eyebrow">Investment session</p>
          <h1>Plan first. Ask safely. Submit deliberately.</h1>
          <p>CHECK sends only capital, account count, IPO names, amounts, and public references.</p>
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

      <section className="studio-grid">
        <div className="form-panel investment-form">
          <fieldset className="ipo-fieldset">
            <legend>Applying for</legend>
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
                    required={ipo.included}
                    value={ipo.name}
                    onChange={(e) => updateIpo(ipo.id, { name: e.target.value })}
                  />
                </label>
                <label>
                  Amount per account (₹)
                  <input
                    inputMode="decimal"
                    required={ipo.included}
                    value={ipo.amountRupees}
                    onChange={(e) => updateIpo(ipo.id, { amountRupees: e.target.value })}
                  />
                </label>
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
                  { id: newId("ipo"), name: "", amountRupees: "", included: true },
                ])
              }
              type="button"
            >
              + Add another IPO
            </button>
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
          {error && <p className="inline-error">{error}</p>}
        </div>

        <aside className="recommendation-panel" aria-live="polite">
          <p className="eyebrow">Recommendation review</p>
          {!recommendation ? (
            <div className="recommendation-empty">
              <span>DEV</span>
              <h2>Nothing leaves the private boundary yet.</h2>
              <p>
                CHECK builds the approved request and runs the deterministic development algorithm.
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
                      {ipo.missing_public_data.length > 0 && (
                        <small>Missing: {ipo.missing_public_data.join(", ")}</small>
                      )}
                    </div>
                  </li>
                ))}
              </ol>
            </div>
          )}
          {message && <p className="success-message">{message}</p>}
        </aside>
      </section>

      <footer className="action-rail" aria-label="Investment actions">
        <div>
          <span className={recommendation ? "rail-step complete" : "rail-step active"}>
            01 CHECK
          </span>
          <span className={recommendation ? "rail-step active" : "rail-step"}>02 EDIT</span>
          <span className="rail-step">03 SUBMIT</span>
        </div>
        <div className="action-buttons">
          <button
            className="secondary-button"
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
      </footer>
    </div>
  );
}

function AllotmentView({ bridge, members }: { bridge: CommandBridge; members: MemberRow[] }) {
  const [candidates, setCandidates] = useState<AllotmentCandidate[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [report, setReport] = useState<AllotmentJobReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    bridge
      .invoke<AllotmentCandidate[]>("list_allotment_candidates")
      .then((rows) => {
        setCandidates(rows);
        if (rows[0]) setSelected(rows[0].application_id);
      })
      .catch((e) => setError(String(e)));
  }, [bridge]);

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
          registrar_name: candidate.registrar_name,
          official_status_url: candidate.official_status_url,
        },
      });
      setReport(result);
      setMessage(`Job ${result.job_id} → ${result.status}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page-content">
      <section className="panel">
        <div className="panel-heading">
          <div>
            <p className="eyebrow">Registrar allotment</p>
            <h1>Check Allotment</h1>
            <p>
              Submitted IPOs only. Fixture KFintech provider for local/synthetic runs. PAN decrypts
              only inside purpose-scoped allotment checks.
            </p>
          </div>
        </div>
        {error && <p className="inline-error">{error}</p>}
        {message && <p className="inline-ok">{message}</p>}
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
                <small>
                  {c.registrar_name} · {c.account_count} account
                  {c.account_count === 1 ? "" : "s"} · {formatRupees(c.planned_amount_paise)}{" "}
                  planned
                </small>
              </span>
            </label>
          ))}
        </div>
        <footer className="action-rail">
          <button
            className="primary-button"
            disabled={!selected || busy}
            onClick={runCheck}
            type="button"
          >
            {busy ? "Checking accounts…" : "Check All Accounts"}
          </button>
        </footer>
      </section>
      {report && (
        <section aria-label="Allotment report card" className="panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Report card</p>
              <h2>{report.ipo_name}</h2>
              <p>
                {report.registrar_name} · {report.provider_id} · {report.status}
              </p>
            </div>
          </div>
          <ul className="stack-list">
            {report.accounts.map((row) => (
              <li key={row.attempt_id}>
                <strong>
                  {row.display_name} · {row.account_kind}
                </strong>
                <small>
                  {row.masked_pan} · {row.status}
                  {row.allotted_lots != null ? ` · ${row.allotted_lots} lot(s)` : ""}
                  {row.allotted_shares != null ? ` / ${row.allotted_shares} shares` : ""}
                </small>
              </li>
            ))}
          </ul>
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
      {boot === "loading" && <div className="loading-bar" aria-label="Loading local vault" />}
      {view === "dashboard" && (
        <DashboardView
          dashboard={dashboard}
          onAllotment={() => setView("allotment")}
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
    </AppShell>
  );
}
