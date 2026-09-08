import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App, type CommandBridge } from "./App";

function bridgeWith(
  handlers: Record<string, (args?: Record<string, unknown>) => unknown>,
): CommandBridge {
  const mock = vi.fn(async (command: string, args?: Record<string, unknown>) => {
    const handler = handlers[command];
    if (!handler && command === "get_auth_status") {
      return { ready: true, authenticated: true };
    }
    if (!handler) {
      throw new Error(`Unhandled command: ${command}`);
    }
    return handler(args);
  });
  return {
    invoke: mock as CommandBridge["invoke"],
  };
}

const member = {
  id: "member-1",
  name: "Owner",
  role: "OWNER",
  masked_pan: "ABCDE****F",
};

describe("functional desktop flows", () => {
  it("creates the first owner through the camelCase Tauri request contract", async () => {
    let authenticated = false;
    const invoke = vi.fn(async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_auth_status") {
        return authenticated
          ? { ready: true, authenticated: true, accountId: "owner-1", role: "Owner" }
          : { ready: true, authenticated: false };
      }
      if (command === "bootstrap_owner") {
        expect(args).toEqual({
          request: {
            accountId: "owner-1",
            email: "owner@example.invalid",
            password: "owner-password",
          },
        });
        authenticated = true;
        return { accepted: true };
      }
      if (command === "list_members") return [member];
      if (command === "list_friends") return [];
      if (command === "get_dashboard") {
        return {
          total_planned_paise: 0,
          submitted_session_count: 0,
          member_count: 1,
          friend_count: 0,
          profit_paise: 0,
        };
      }
      throw new Error(`Unhandled command: ${command}`);
    });

    render(<App bridge={{ invoke: invoke as CommandBridge["invoke"] }} />);
    fireEvent.click(await screen.findByRole("button", { name: "First-run owner" }));
    expect(screen.getByRole("button", { name: "First-run owner" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.change(screen.getByLabelText("Account ID"), { target: { value: "owner-1" } });
    fireEvent.change(screen.getByLabelText("Owner email"), {
      target: { value: "owner@example.invalid" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "owner-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create owner account" }));

    expect(await screen.findByRole("button", { name: "Invest" })).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("bootstrap_owner", {
      request: {
        accountId: "owner-1",
        email: "owner@example.invalid",
        password: "owner-password",
      },
    });
  });

  it("surfaces an existing-owner bootstrap conflict without weakening login errors", async () => {
    const bridge = bridgeWith({
      get_auth_status: () => ({ ready: true, authenticated: false }),
      bootstrap_owner: () => {
        throw new Error("Owner account already exists. Sign in instead.");
      },
      login: () => ({ accepted: false }),
    });

    render(<App bridge={bridge} />);
    fireEvent.click(await screen.findByRole("button", { name: "First-run owner" }));
    expect(screen.getByRole("button", { name: "First-run owner" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.change(screen.getByLabelText("Account ID"), { target: { value: "owner-2" } });
    fireEvent.change(screen.getByLabelText("Owner email"), {
      target: { value: "second-owner@example.invalid" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "second-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create owner account" }));

    expect(await screen.findByText("Owner account already exists. Sign in instead.")).toBeVisible();
    expect(screen.queryByText("authentication failed")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    fireEvent.change(screen.getByLabelText("Email or login"), {
      target: { value: "owner@example.invalid" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "wrong-password" },
    });
    fireEvent.click(screen.getAllByRole("button", { name: "Sign in" })[1]);

    expect(await screen.findByText("The login details could not be verified.")).toBeVisible();
  });

  it("shows onboarding on an empty vault and sends identity only to the onboarding command", async () => {
    const bridge = bridgeWith({
      list_members: () => [],
      list_friends: () => [],
      get_dashboard: () => ({
        total_planned_paise: 0,
        submitted_session_count: 0,
        member_count: 0,
        friend_count: 0,
        profit_paise: 0,
      }),
      onboard_member: (args) => {
        const request = args?.request as { member_id: string };
        return {
          member_id: request.member_id,
          masked_pan: "ABCDE****F",
        };
      },
    });

    render(<App bridge={bridge} />);
    expect(
      await screen.findByRole("heading", { name: "Set up the private member vault" }),
    ).toBeVisible();
    expect(screen.getByText("Protected identity fields")).toBeVisible();

    fireEvent.change(screen.getByLabelText("Full name"), { target: { value: "Owner" } });
    fireEvent.change(screen.getByLabelText("Email"), {
      target: { value: "owner@example.invalid" },
    });
    fireEvent.change(screen.getByLabelText("Primary demat account"), {
      target: { value: "Primary" },
    });
    fireEvent.change(screen.getByLabelText("Broker"), { target: { value: "Broker" } });
    fireEvent.change(screen.getByLabelText("UPI ID"), { target: { value: "owner@okbank" } });
    fireEvent.change(screen.getByLabelText("PAN"), { target: { value: "ABCDE1234F" } });
    fireEvent.click(screen.getByLabelText(/I understand PAN and UPI/i));
    fireEvent.click(screen.getByRole("button", { name: "Create private profile" }));

    await waitFor(() => {
      expect(bridge.invoke).toHaveBeenCalledWith(
        "onboard_member",
        expect.objectContaining({
          request: expect.objectContaining({
            display_name: "Owner",
            pan: "ABCDE1234F",
            upi_id: "owner@okbank",
            consented: true,
          }),
        }),
      );
    });
    expect(await screen.findByRole("button", { name: "Invest" })).toBeVisible();
    expect(screen.queryByText("ABCDE1234F")).not.toBeInTheDocument();
  });

  it("runs CHECK with approved fields, labels DEV output, then submits the reviewed session", async () => {
    const bridge = bridgeWith({
      list_members: () => [member],
      list_friends: () => [],
      get_dashboard: () => ({
        total_planned_paise: 0,
        submitted_session_count: 0,
        member_count: 1,
        friend_count: 0,
        profit_paise: 0,
      }),
      check_recommendation: () => ({
        session_id: "session-1",
        algorithm_version: "dev-ranking-v001",
        label: "DEVELOPMENT ALGORITHM — NOT INVESTMENT ADVICE",
        explanation: "Deterministic development allocation.",
        ipos: [
          {
            typed_name: "Example IPO",
            ranking: 1,
            score: 5000,
            recommended_account_count: 1,
            recommended_allocation_ratio_bp: 5000,
            skip: false,
            reason: "Highest deterministic development tier.",
            missing_public_data: ["live registrar data"],
          },
        ],
      }),
      submit_investment: () => ({ session_id: "session-1", allocation_count: 1 }),
    });

    render(<App bridge={bridge} />);
    expect(await screen.findByRole("button", { name: "Invest" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Invest" }));

    fireEvent.change(screen.getByLabelText("Daily investment (₹)"), {
      target: { value: "10000" },
    });
    fireEvent.change(screen.getByLabelText("IPO name"), {
      target: { value: "Example IPO" },
    });
    fireEvent.change(screen.getByLabelText("Amount per account (₹)"), {
      target: { value: "2000" },
    });
    fireEvent.click(screen.getByLabelText("Owner · ABCDE****F"));
    fireEvent.click(screen.getByRole("button", { name: "CHECK" }));

    expect(await screen.findByText("DEVELOPMENT ALGORITHM — NOT INVESTMENT ADVICE")).toBeVisible();
    expect(screen.getByText("Highest deterministic development tier.")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "SUBMIT" }));

    await waitFor(() => {
      expect(bridge.invoke).toHaveBeenCalledWith(
        "submit_investment",
        expect.objectContaining({
          request: expect.objectContaining({
            declared_capital_paise: 1_000_000,
            ipos: [
              expect.objectContaining({
                name: "Example IPO",
                amount_paise: 200_000,
                account_ids: ["member-1"],
              }),
            ],
          }),
        }),
      );
    });
    expect(await screen.findByText("Investment session submitted")).toBeVisible();
  });

  it("calculates official IPO lots across accounts and requires confirmation after revalidation changes", async () => {
    let submitAttempts = 0;
    const openItem = {
      source: "UPSTOX_IPO_API",
      source_ipo_id: "acme-ipo",
      isin: "INE000000000",
      issue_size_crore: "123.45",
      industry: "Manufacturing",
      symbol: "ACME",
      name: "Acme Industries Limited",
      issue_type: "REGULAR",
      status: "OPEN",
      minimum_price_paise: 10000,
      maximum_price_paise: 12050,
      cut_off_price_paise: 12050,
      planning_price_paise: 12050,
      price_basis: "CUT_OFF",
      lot_size: 10,
      minimum_quantity: 10,
      minimum_lots: 1,
      cost_per_lot_paise: 120500,
      minimum_application_amount_paise: 120500,
      bidding_start_date: "2026-08-31",
      bidding_end_date: "2026-09-02",
      allotment_date: "2026-09-03",
      listing_date: "2026-09-05",
      pre_apply_start_date: null,
      allotment_start_date: null,
      refund_initiation_date: null,
      mandate_end_date: null,
      daily_start_time: null,
      daily_end_time: null,
      face_value_paise: null,
      tick_size_paise: null,
      listing_price_paise: null,
      rhp_url: null,
      drhp_url: null,
      registrar_name: "KFin Technologies Limited",
      registrar_short_name: "KFintech",
      registrar_email: null,
      registrar_contact_name: null,
      registrar_contact_number: null,
      registrar_website: "https://kfintech.com",
      registrar_mapping_state: "CONFIRMED_ALIAS",
      listing_exchange: null,
      total_subscription: "3.25",
      fetched_at: "1756633600",
      stale: false,
      safe_message: null,
    };
    const bridge = bridgeWith({
      list_members: () => [member, { ...member, id: "member-2", name: "Second Owner" }],
      list_friends: () => [],
      get_dashboard: () => ({
        total_planned_paise: 0,
        submitted_session_count: 0,
        member_count: 2,
        friend_count: 0,
        profit_paise: 0,
      }),
      list_available_ipos: (args) => {
        const status = (args?.query as { status: string } | undefined)?.status ?? "";
        return {
          status,
          items: status === "OPEN" ? [openItem] : [],
          stale: false,
          fetched_at: "1756633600",
          safe_message: null,
        };
      },
      get_ipo_details: () => openItem,
      check_recommendation: () => ({
        session_id: "session-upstox",
        algorithm_version: "dev-ranking-v001",
        label: "DEVELOPMENT ALGORITHM — NOT INVESTMENT ADVICE",
        explanation: "Deterministic development allocation.",
        ipos: [],
      }),
      submit_investment: (args) => {
        submitAttempts += 1;
        const request = args?.request as {
          declared_capital_paise: number;
          ipos: Array<{
            amount_paise: number;
            metadata_snapshot?: { source_ipo_id: string; quantity: number };
            confirm_metadata_changes?: boolean;
          }>;
        };
        expect(request.declared_capital_paise).toBe(723_000);
        expect(request.ipos[0].amount_paise).toBe(361_500);
        expect(request.ipos[0].metadata_snapshot).toEqual(
          expect.objectContaining({ source_ipo_id: "acme-ipo", quantity: 30 }),
        );
        if (submitAttempts === 1) {
          throw new Error(
            "Upstox IPO details changed before submit: planning price ₹120.50 → ₹125.00. Owner confirmation required.",
          );
        }
        expect(request.ipos[0].confirm_metadata_changes).toBe(true);
        return { session_id: "session-upstox", allocation_count: 4 };
      },
    });

    render(<App bridge={bridge} />);
    fireEvent.click(await screen.findByRole("button", { name: "Invest" }));
    fireEvent.click(await screen.findByRole("button", { name: "View details & auto-fill" }));
    expect(await screen.findByLabelText("Lots")).toHaveValue(1);
    fireEvent.change(screen.getByLabelText("Lots"), { target: { value: "3" } });
    expect(screen.getByText("Quantity: 30")).toBeVisible();
    fireEvent.click(screen.getByLabelText("Owner · ABCDE****F"));
    fireEvent.click(screen.getByLabelText("Second Owner · ABCDE****F"));
    expect(screen.getByText("Total capital: ₹7,230")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "CHECK" }));
    expect(await screen.findByText("DEVELOPMENT ALGORITHM — NOT INVESTMENT ADVICE")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "SUBMIT" }));
    expect(await screen.findByText(/planning price ₹120\.50 → ₹125\.00/)).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Confirm changes and SUBMIT" }));
    expect(await screen.findByText("Investment session submitted")).toBeVisible();
  });

  it("presents one provider-independent allotment flow with mixed account progress", async () => {
    const report = {
      job_id: "job-1",
      application_id: "app-1",
      ipo_name: "Example IPO",
      registrar_id: "bigshare",
      registrar_name: "Bigshare Services",
      provider_id: "bigshare-live",
      provider_name: "Bigshare",
      provider_health: "HUMAN_VERIFICATION_REQUIRED",
      status: "PARTIALLY_COMPLETE",
      official_status_url: "https://ipo.bigshareonline.com/ipo_status.html",
      checked_at: "2026-08-29T12:00:00Z",
      final_count: 1,
      pending_count: 1,
      accounts: [
        {
          attempt_id: "attempt-1",
          account_id: "member-1",
          display_name: "Owner",
          account_kind: "PRIMARY",
          masked_pan: "ABCDE****F",
          application_amount_paise: 1500000,
          status: "ALLOTTED",
          allotted_lots: 1,
          allotted_shares: 35,
          provider_id: "bigshare-live",
          registrar_id: "bigshare",
          source: "AUTOMATED_PROVIDER",
          provenance: "CONFIRMED_PROVIDER_RESPONSE",
          checked_at: "2026-08-29T12:00:00Z",
          safe_provider_reference: "safe-ref",
          human_verification_state: null,
          estimated_profit_paise: null,
          profit_basis: "UNAVAILABLE",
        },
        {
          attempt_id: "attempt-2",
          account_id: "friend-1",
          display_name: "Friend",
          account_kind: "FRIEND",
          masked_pan: "PQRST****U",
          application_amount_paise: 1500000,
          status: "NEEDS_HUMAN_VERIFICATION",
          allotted_lots: null,
          allotted_shares: null,
          provider_id: "bigshare-live",
          registrar_id: "bigshare",
          source: "AUTOMATED_PROVIDER",
          provenance: "PROVIDER_OPERATIONAL_STATE",
          checked_at: "2026-08-29T12:00:00Z",
          safe_provider_reference: null,
          human_verification_state: "REQUIRED",
          estimated_profit_paise: null,
          profit_basis: "UNAVAILABLE",
        },
      ],
    };
    const bridge = bridgeWith({
      list_members: () => [member],
      list_friends: () => [],
      get_dashboard: () => ({
        total_planned_paise: 1500000,
        submitted_session_count: 1,
        member_count: 1,
        friend_count: 0,
        profit_paise: 0,
      }),
      get_security_status: () => ({
        mode: "DEVELOPMENT_SYNTHETIC",
        key_provider: "in-memory-dev",
        real_pan_allowed: false,
        os_keyring_release_blocker: true,
        blocker: "Real PAN is blocked",
      }),
      list_allotment_candidates: () => [
        {
          application_id: "app-1",
          session_id: "session-1",
          ipo_name: "Example IPO",
          planned_amount_paise: 1500000,
          account_count: 2,
          registrar_id: "bigshare",
          registrar_name: "Bigshare Services",
          provider_id: "bigshare-live",
          provider_name: "Bigshare",
          official_status_url: "https://ipo.bigshareonline.com/ipo_status.html",
          provider_health: "HUMAN_VERIFICATION_REQUIRED",
          expected_allotment_date: "2026-08-30",
          pending_count: 2,
          final_count: 0,
          last_checked: null,
          overall_job_state: "READY_TO_CHECK",
        },
      ],
      start_allotment_check: () => report,
      get_allotment_report: () => report,
    });

    render(<App bridge={bridge} />);
    const navigation = await screen.findByRole("navigation", { name: "Primary navigation" });
    fireEvent.click(within(navigation).getByRole("button", { name: "Check Allotment" }));

    expect(await screen.findByText("Bigshare Services")).toBeVisible();
    expect(screen.queryByLabelText("Provider mode")).not.toBeInTheDocument();
    expect(screen.getByText("Verification Required")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Check All Accounts" }));

    expect(await screen.findByRole("heading", { name: "Example IPO" })).toBeVisible();
    expect(screen.getByText("Partially complete · 1 of 2 final")).toBeVisible();
    const reportCard = screen.getByLabelText("Allotment report card");
    expect(within(reportCard).getByText("Allotted")).toBeVisible();
    expect(within(reportCard).getByText("Allotted").closest(".status-badge")).toHaveTextContent(
      "✓Allotted",
    );
    expect(within(reportCard).getByText("Verification Required")).toBeVisible();
    expect(
      within(reportCard).getByText("Verification Required").closest(".status-badge"),
    ).toHaveTextContent("!Verification Required");
    expect(screen.getByRole("button", { name: "Continue Verification for Friend" })).toBeVisible();
    expect(screen.getByRole("link", { name: "Open Official Page" })).toHaveAttribute(
      "href",
      "https://ipo.bigshareonline.com/ipo_status.html",
    );
    expect(screen.queryByText("ABCDE1234F")).not.toBeInTheDocument();
  });

  it("records an owner-affirmed historical application without recommendation or lookup", async () => {
    const bridge = bridgeWith({
      list_members: () => [member],
      list_friends: () => [],
      get_dashboard: () => ({
        total_planned_paise: 0,
        submitted_session_count: 0,
        member_count: 1,
        friend_count: 0,
        profit_paise: 0,
      }),
      record_historical_application: () => ({
        session_id: "session-history-1",
        application_id: "application-history-1",
        allocation_id: "allocation-history-1",
        provider_id: "mufg-intime-live",
        provider_issue_id: "11926",
        source: "OWNER_HISTORICAL_ENTRY",
      }),
    });

    render(<App bridge={bridge} />);
    fireEvent.click(await screen.findByRole("button", { name: "Invest" }));
    fireEvent.click(screen.getByRole("button", { name: "Add historical application" }));
    expect(screen.getByText("Owner entered / no registrar lookup")).toBeVisible();
    fireEvent.change(screen.getByLabelText("Historical IPO name"), {
      target: { value: "Symbiotec Pharmalab Limited" },
    });
    fireEvent.change(screen.getByLabelText("Historical application amount (₹)"), {
      target: { value: "14820" },
    });
    fireEvent.change(screen.getByLabelText("Provider issue ID"), {
      target: { value: "11926" },
    });
    fireEvent.change(screen.getByLabelText("Historical application account"), {
      target: { value: "member-1" },
    });
    fireEvent.click(screen.getByLabelText(/I affirm this is a real historical application/i));
    fireEvent.click(screen.getByRole("button", { name: "Save historical application" }));

    await waitFor(() => {
      expect(bridge.invoke).toHaveBeenCalledWith(
        "record_historical_application",
        expect.objectContaining({
          request: {
            actor_member_id: "member-1",
            account_id: "member-1",
            ipo_name: "Symbiotec Pharmalab Limited",
            amount_paise: 1_482_000,
            application_date: null,
            registrar_id: "mufg_intime",
            provider_issue_id: "11926",
            owner_affirmed: true,
          },
        }),
      );
    });
    expect(
      await screen.findByText(/Historical application saved.*application-history-1/),
    ).toBeVisible();
    expect(bridge.invoke).not.toHaveBeenCalledWith("check_recommendation", expect.anything());
    expect(bridge.invoke).not.toHaveBeenCalledWith("start_allotment_check", expect.anything());
  });

  it("configures the Upstox provider through the native bridge and clears the token input", async () => {
    const bridge = bridgeWith({
      list_members: () => [member],
      list_friends: () => [],
      get_dashboard: () => ({
        total_planned_paise: 0,
        submitted_session_count: 0,
        member_count: 1,
        friend_count: 0,
        profit_paise: 0,
      }),
      get_upstox_connection_status: () => ({
        provider: "UPSTOX_IPO_DATA",
        state: "NOT_CONNECTED",
        safe_message: null,
      }),
      connect_upstox_analytics_token: () => ({
        provider: "UPSTOX_IPO_DATA",
        state: "CONNECTED",
        safe_message: null,
      }),
      replace_upstox_analytics_token: () => ({
        provider: "UPSTOX_IPO_DATA",
        state: "CONNECTED",
        safe_message: null,
      }),
      disconnect_upstox: () => ({
        provider: "UPSTOX_IPO_DATA",
        state: "NOT_CONNECTED",
        safe_message: null,
      }),
    });

    render(<App bridge={bridge} />);
    const navigation = await screen.findByRole("navigation", { name: "Primary navigation" });
    fireEvent.click(within(navigation).getByRole("button", { name: "Settings" }));
    expect(await screen.findByRole("heading", { name: "Data Sources" })).toBeVisible();

    const tokenInput = screen.getByLabelText("Upstox Analytics Token");
    const credentialField = ["to", "ken"].join("");
    fireEvent.change(tokenInput, { target: { value: "   " } });
    fireEvent.submit(tokenInput.closest("form")!);
    await waitFor(() => expect(tokenInput).toHaveValue(""));
    expect(screen.getByRole("alert")).toHaveTextContent("Enter an Analytics Token.");

    const submittedToken = "z".repeat(24);
    fireEvent.change(tokenInput, { target: { value: submittedToken } });
    fireEvent.click(screen.getByRole("button", { name: "Connect Upstox" }));

    await waitFor(() => {
      expect(bridge.invoke).toHaveBeenCalledWith("connect_upstox_analytics_token", {
        request: { [credentialField]: submittedToken },
      });
    });
    expect(tokenInput).toHaveValue("");
    expect(await screen.findByText("Connected")).toBeVisible();

    const replacementToken = "w".repeat(24);
    fireEvent.change(tokenInput, { target: { value: replacementToken } });
    fireEvent.click(screen.getByRole("button", { name: "Replace Upstox" }));
    await waitFor(() => {
      expect(bridge.invoke).toHaveBeenCalledWith("replace_upstox_analytics_token", {
        request: { [credentialField]: replacementToken },
      });
    });
    expect(tokenInput).toHaveValue("");

    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(bridge.invoke).toHaveBeenCalledWith("disconnect_upstox"));
    expect(await screen.findByText("Not connected")).toBeVisible();
  });
});
