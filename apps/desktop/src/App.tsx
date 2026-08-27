const navigation = ["Dashboard", "Members", "Investments", "News", "Report", "Strategy Lab"];

const metrics = [
  { label: "Group investment", value: "₹0", detail: "No submitted sessions yet" },
  { label: "Group net profit", value: "₹0", detail: "After friend-share deductions" },
  { label: "Active applications", value: "0", detail: "All accounts included" },
  { label: "Allotment rate", value: "—", detail: "Waiting for first finalized result" },
];

function Mark() {
  return (
    <span className="brand-mark" aria-hidden="true">
      S
    </span>
  );
}

function MenuIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 24 24">
      <circle cx="5" cy="12" r="1.8" />
      <circle cx="12" cy="12" r="1.8" />
      <circle cx="19" cy="12" r="1.8" />
    </svg>
  );
}

function ArrowIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 24 24">
      <path d="m5 12 14 0M13 6l6 6-6 6" />
    </svg>
  );
}

export function App() {
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
          <p className="eyebrow">Workspace</p>
          <ul>
            {navigation.map((item, index) => (
              <li key={item}>
                <button className={index === 0 ? "nav-item active" : "nav-item"} type="button">
                  <span className="nav-dot" aria-hidden="true" />
                  {item}
                </button>
              </li>
            ))}
          </ul>
        </nav>

        <div className="privacy-card">
          <span className="privacy-indicator" aria-hidden="true" />
          <div>
            <strong>Private domain locked</strong>
            <span>External AI has no member-vault access</span>
          </div>
        </div>

        <button className="profile-button" type="button">
          <span className="avatar" aria-hidden="true">
            CO
          </span>
          <span>
            <strong>Core member</strong>
            <small>Local profile</small>
          </span>
          <MenuIcon />
        </button>
      </aside>

      <main>
        <header className="topbar">
          <div>
            <span className="saved-status">
              <span aria-hidden="true" /> Saved locally
            </span>
            <span className="sync-status">Pending sync</span>
          </div>
          <button className="menu-button" aria-label="Open application menu" type="button">
            <MenuIcon />
          </button>
        </header>

        <div className="dashboard">
          <section className="hero" aria-labelledby="dashboard-heading">
            <div>
              <p className="eyebrow">Friday · Group overview</p>
              <h1 id="dashboard-heading">Capital, clarity, and every account in view.</h1>
              <p className="hero-copy">
                Local-first investment tracking with allocation-level accounting and
                private-by-design workflows.
              </p>
            </div>
            <div className="quick-actions" aria-label="Quick actions">
              <button aria-label="Invest" className="primary-action" type="button">
                <span>Invest</span>
                <small>Plan or submit a session</small>
                <ArrowIcon />
              </button>
              <button aria-label="Check Allotment" className="secondary-action" type="button">
                <span>Check Allotment</span>
                <small>Review eligible applications</small>
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
            <article className="panel performance-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Capital movement</p>
                  <h2>Investment activity</h2>
                </div>
                <button type="button">90 days</button>
              </div>
              <div className="empty-chart" aria-label="No investment activity yet">
                <div className="chart-grid" aria-hidden="true" />
                <div className="empty-copy">
                  <span>01</span>
                  <strong>Your first submitted allocation will appear here.</strong>
                  <p>Drafts remain local and private until you choose Submit.</p>
                </div>
              </div>
            </article>

            <article className="panel activity-panel">
              <div className="panel-heading">
                <div>
                  <p className="eyebrow">Audit timeline</p>
                  <h2>Recent activity</h2>
                </div>
              </div>
              <div className="activity-empty">
                <span className="activity-glyph" aria-hidden="true">
                  ✓
                </span>
                <strong>Foundation ready</strong>
                <p>Verified local events will appear here with actor, device, and timestamp.</p>
              </div>
            </article>
          </section>
        </div>
      </main>
    </div>
  );
}
