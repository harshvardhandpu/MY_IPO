import { create } from "zustand";
import { persist } from "zustand/middleware";
import { IPOS, getIpo, minInvest } from "@/lib/ipo-data";
import { hashSeed } from "@/lib/utils";

export type AccountKind = "PRIMARY" | "FRIEND";

export type Account = {
  id: string;
  label: string;
  kind: AccountKind;
  last4: string;
  shareBps: number;
  eligible: boolean;
  archived: boolean;
};

export type ResultStatus =
  | "PENDING"
  | "CHECKING"
  | "ALLOTTED"
  | "NOT_ALLOTTED"
  | "NOT_FOUND"
  | "UNKNOWN"
  | "NEEDS_HUMAN"
  | "RATE_LIMITED"
  | "PROVIDER_UNAVAILABLE"
  | "MANUAL";

export type Provenance =
  | "UNCHECKED"
  | "SIMULATED_PROVIDER"
  | "MANUAL"
  | "HISTORICAL";

export type Allocation = {
  id: string;
  accountId: string;
  lots: number;
  amountPaise: number;
  status: ResultStatus;
  allottedLots: number;
  allottedShares: number;
  provenance: Provenance;
  checkedAt?: string;
  note?: string;
};

export type Application = {
  id: string;
  ipoSlug: string;
  registrar: string;
  source: "INVEST" | "HISTORICAL";
  createdAt: string;
  applicationDate?: string;
  voided: boolean;
  allocations: Allocation[];
};

function uid(prefix: string) {
  return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

function nowIso() {
  return new Date().toISOString();
}

export function rupeesToPaise(n: number) {
  return Math.round(n * 100);
}

export function paiseToRupees(n: number) {
  return n / 100;
}

export function maskLast4(last4: string) {
  const t = last4.trim().toUpperCase();
  return t ? `•••• ${t}` : "••••";
}

export function providerForRegistrar(name: string) {
  const n = name.toLowerCase();
  if (n.includes("kfin")) return { id: "kfintech-live", label: "KFintech", auto: true };
  if (n.includes("mufg") || n.includes("intime") || n.includes("link"))
    return { id: "mufg-intime-live", label: "MUFG Intime", auto: true };
  if (n.includes("bigshare")) return { id: "bigshare-live", label: "Bigshare", auto: false };
  return { id: "unsupported", label: name, auto: false };
}

export function simulateAccountCheck(
  registrar: string,
  accountId: string,
  ipoSlug: string,
): Pick<Allocation, "status" | "allottedLots" | "note" | "provenance"> {
  const provider = providerForRegistrar(registrar);
  const seed = hashSeed(`${accountId}:${ipoSlug}:${registrar}`);
  if (provider.id === "bigshare-live") {
    return {
      status: "NEEDS_HUMAN",
      allottedLots: 0,
      provenance: "SIMULATED_PROVIDER",
      note: "Official portal requires human verification. Not treated as not allotted.",
    };
  }
  if (provider.id === "unsupported") {
    return {
      status: "UNKNOWN",
      allottedLots: 0,
      provenance: "UNCHECKED",
      note: "No automated transport. Enter a manual result.",
    };
  }
  const roll = seed % 100;
  if (provider.id === "mufg-intime-live" && roll < 28) {
    return {
      status: "NEEDS_HUMAN",
      allottedLots: 0,
      provenance: "SIMULATED_PROVIDER",
      note: "CAPTCHA state requires owner continuation.",
    };
  }
  if (roll < 8) {
    return {
      status: "RATE_LIMITED",
      allottedLots: 0,
      provenance: "SIMULATED_PROVIDER",
      note: "502 / rate limit — retryable, not a financial fact.",
    };
  }
  if (roll < 16) {
    return {
      status: "NOT_FOUND",
      allottedLots: 0,
      provenance: "SIMULATED_PROVIDER",
      note: "404 unresolved. Not classified as not allotted.",
    };
  }
  if (roll < 55) {
    return {
      status: "NOT_ALLOTTED",
      allottedLots: 0,
      provenance: "SIMULATED_PROVIDER",
      note: "Authoritative zero-share record.",
    };
  }
  return {
    status: "ALLOTTED",
    allottedLots: seed % 3 === 0 ? 2 : 1,
    provenance: "SIMULATED_PROVIDER",
    note: "Simulated provider response. Official registrar remains source of truth.",
  };
}

type Ledger = {
  accounts: Account[];
  applications: Application[];
  addFriend: (input: { label: string; last4: string; shareBps: number; eligible: boolean }) => void;
  archiveAccount: (id: string) => void;
  setShare: (id: string, shareBps: number, eligible: boolean) => void;
  submitInvestment: (input: {
    ipoSlug: string;
    accountIds: string[];
    lots: number;
    source: "INVEST" | "HISTORICAL";
    applicationDate?: string;
  }) => string;
  voidApplication: (id: string) => void;
  runAllotment: (applicationId: string) => void;
  setManualResult: (
    applicationId: string,
    allocationId: string,
    status: "ALLOTTED" | "NOT_ALLOTTED",
    lots: number,
  ) => void;
};

const seedAccounts: Account[] = [
  {
    id: "acct-owner",
    label: "Harshvardhan",
    kind: "PRIMARY",
    last4: "234H",
    shareBps: 0,
    eligible: false,
    archived: false,
  },
  {
    id: "acct-family",
    label: "Family book",
    kind: "FRIEND",
    last4: "881A",
    shareBps: 1000,
    eligible: true,
    archived: false,
  },
];

function seedAlloc(
  id: string,
  accountId: string,
  ipoSlug: string,
  lots: number,
  extra: Partial<Allocation> = {},
): Allocation {
  const ipo = getIpo(ipoSlug);
  const amountPaise = rupeesToPaise(minInvest(ipo ?? IPOS[0]) * lots);
  return {
    id,
    accountId,
    lots,
    amountPaise,
    status: "PENDING",
    allottedLots: 0,
    allottedShares: 0,
    provenance: "UNCHECKED",
    ...extra,
  };
}

const seedApplications: Application[] = [
  {
    id: "app-ss-retail",
    ipoSlug: "ss-retail",
    registrar: "KFin Technologies",
    source: "INVEST",
    createdAt: "2026-09-16T08:10:00.000Z",
    voided: false,
    allocations: [
      seedAlloc("alloc-ss-owner", "acct-owner", "ss-retail", 1),
      seedAlloc("alloc-ss-family", "acct-family", "ss-retail", 1),
    ],
  },
  {
    id: "app-manika",
    ipoSlug: "manika-plastech",
    registrar: "Bigshare Services",
    source: "INVEST",
    createdAt: "2026-09-15T11:40:00.000Z",
    voided: false,
    allocations: [seedAlloc("alloc-manika-owner", "acct-owner", "manika-plastech", 1)],
  },
  {
    id: "app-rento",
    ipoSlug: "rentomojo",
    registrar: "KFin Technologies",
    source: "HISTORICAL",
    createdAt: "2026-09-12T07:00:00.000Z",
    applicationDate: "2026-09-08",
    voided: false,
    allocations: [
      seedAlloc("alloc-rento-owner", "acct-owner", "rentomojo", 1, {
        status: "ALLOTTED",
        allottedLots: 1,
        allottedShares: 101,
        provenance: "HISTORICAL",
        checkedAt: "2026-09-12T07:00:00.000Z",
        note: "Owner-entered historical allotment. No provider lookup was made.",
      }),
    ],
  },
];

export const useLedger = create<Ledger>()(
  persist(
    (set, get) => ({
      accounts: seedAccounts,
      applications: seedApplications,
      addFriend: ({ label, last4, shareBps, eligible }) => {
        set((s) => ({
          accounts: [
            ...s.accounts,
            {
              id: uid("acct"),
              label: label.trim(),
              kind: "FRIEND",
              last4: last4.trim().toUpperCase(),
              shareBps,
              eligible,
              archived: false,
            },
          ],
        }));
      },
      archiveAccount: (id) =>
        set((s) => ({
          accounts: s.accounts.map((a) =>
            a.id === id && a.kind === "FRIEND" ? { ...a, archived: true } : a,
          ),
        })),
      setShare: (id, shareBps, eligible) =>
        set((s) => ({
          accounts: s.accounts.map((a) => (a.id === id ? { ...a, shareBps, eligible } : a)),
        })),
      submitInvestment: ({ ipoSlug, accountIds, lots, source, applicationDate }) => {
        const ipo = getIpo(ipoSlug);
        if (!ipo) throw new Error("Issue is not on the desk");
        const amountPaise = rupeesToPaise(minInvest(ipo) * lots);
        const id = uid("app");
        const allocations: Allocation[] = accountIds.map((accountId) => ({
          id: uid("alloc"),
          accountId,
          lots,
          amountPaise,
          status: "PENDING",
          allottedLots: 0,
          allottedShares: 0,
          provenance: source === "HISTORICAL" ? "HISTORICAL" : "UNCHECKED",
        }));
        set((s) => ({
          applications: [
            {
              id,
              ipoSlug,
              registrar: ipo.registrar,
              source,
              createdAt: nowIso(),
              applicationDate,
              voided: false,
              allocations,
            },
            ...s.applications,
          ],
        }));
        return id;
      },
      voidApplication: (id) =>
        set((s) => ({
          applications: s.applications.map((a) => (a.id === id ? { ...a, voided: true } : a)),
        })),
      runAllotment: (applicationId) => {
        const ipoSlug = get().applications.find((a) => a.id === applicationId)?.ipoSlug;
        const ipo = ipoSlug ? getIpo(ipoSlug) : undefined;
        set((s) => ({
          applications: s.applications.map((app) => {
            if (app.id !== applicationId || app.voided) return app;
            return {
              ...app,
              allocations: app.allocations.map((row) => {
                if (
                  row.status === "ALLOTTED" ||
                  row.status === "NOT_ALLOTTED" ||
                  row.status === "MANUAL"
                ) {
                  return row;
                }
                const sim = simulateAccountCheck(app.registrar, row.accountId, app.ipoSlug);
                const shares = (sim.allottedLots ?? 0) * (ipo?.lot ?? 0);
                return {
                  ...row,
                  ...sim,
                  allottedShares: shares,
                  allottedLots: sim.allottedLots,
                  checkedAt: nowIso(),
                };
              }),
            };
          }),
        }));
      },
      setManualResult: (applicationId, allocationId, status, lots) => {
        const app = get().applications.find((a) => a.id === applicationId);
        const ipo = app ? getIpo(app.ipoSlug) : undefined;
        set((s) => ({
          applications: s.applications.map((a) => {
            if (a.id !== applicationId) return a;
            return {
              ...a,
              allocations: a.allocations.map((row) =>
                row.id === allocationId
                  ? {
                      ...row,
                      status: "MANUAL",
                      allottedLots: status === "ALLOTTED" ? lots : 0,
                      allottedShares: status === "ALLOTTED" ? lots * (ipo?.lot ?? 0) : 0,
                      provenance: "MANUAL",
                      checkedAt: nowIso(),
                      note:
                        status === "ALLOTTED"
                          ? "Manual result entered by owner."
                          : "Manual not allotted. Confirmed by owner.",
                    }
                  : row,
              ),
            };
          }),
        }));
      },
    }),
    { name: "sanket-ledger-v2" },
  ),
);

export function plannedPaise(app: Application) {
  return app.allocations.reduce((s, a) => s + a.amountPaise, 0);
}

export function isFinalStatus(status: ResultStatus) {
  return status === "ALLOTTED" || status === "NOT_ALLOTTED" || status === "MANUAL";
}

export function jobState(app: Application): string {
  if (app.voided) return "CANCELLED";
  const statuses = app.allocations.map((a) => a.status);
  if (statuses.every((s) => s === "PENDING")) return "READY";
  if (statuses.some((s) => s === "NEEDS_HUMAN")) return "NEEDS_HUMAN";
  if (statuses.some((s) => s === "RATE_LIMITED")) return "RATE_LIMITED";
  if (statuses.some((s) => s === "PROVIDER_UNAVAILABLE")) return "PROVIDER_UNAVAILABLE";
  if (statuses.some((s) => s === "NOT_FOUND" || s === "UNKNOWN" || s === "CHECKING")) {
    return "UNKNOWN";
  }
  if (statuses.every(isFinalStatus)) {
    const won = app.allocations.some((a) => a.allottedShares > 0);
    return won ? "ALLOTTED" : "NOT_ALLOTTED";
  }
  return "PARTIAL";
}

export function estimatedProfitPaise(app: Application) {
  const ipo = getIpo(app.ipoSlug);
  if (!ipo) return null;
  let total = 0;
  let any = false;
  for (const row of app.allocations) {
    if (row.allottedShares <= 0) continue;
    any = true;
    const gross = rupeesToPaise(row.allottedShares * ipo.gmp);
    const acct = useLedger.getState().accounts.find((a) => a.id === row.accountId);
    const share =
      acct?.eligible && gross > 0 ? Math.floor((gross * acct.shareBps) / 10000) : 0;
    total += gross - share;
  }
  return any ? total : null;
}

export function summarizeDesk(applications: Application[], accounts: Account[]) {
  const live = applications.filter((a) => !a.voided);
  const planned = live.reduce((s, a) => s + plannedPaise(a), 0);
  const profits = live
    .map((a) => estimatedProfitPaise(a))
    .filter((n): n is number => n != null);
  const members = accounts.filter((a) => !a.archived);
  return {
    live,
    plannedPaise: planned,
    profitPaise: profits.length ? profits.reduce((s, n) => s + n, 0) : null,
    sessions: live.length,
    members: members.length,
    friends: members.filter((a) => a.kind === "FRIEND").length,
    pendingChecks: live.filter((a) =>
      a.allocations.some((row) => row.status === "PENDING" || row.status === "NEEDS_HUMAN"),
    ).length,
  };
}

export function officialUrl(registrar: string) {
  const n = registrar.toLowerCase();
  if (n.includes("kfin")) return "https://ipostatus.kfintech.com/";
  if (n.includes("bigshare")) return "https://ipo.bigshareonline.com/IPO_Status.html";
  if (n.includes("skyline")) return "https://www.skylinerta.com/ipo.php";
  return "https://linkintime.co.in/initial_offer/public-issues.html";
}
