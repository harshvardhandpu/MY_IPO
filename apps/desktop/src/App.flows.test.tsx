import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App, type CommandBridge } from "./App";

function bridgeWith(
  handlers: Record<string, (args?: Record<string, unknown>) => unknown>,
): CommandBridge {
  const mock = vi.fn(async (command: string, args?: Record<string, unknown>) => {
    const handler = handlers[command];
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
    fireEvent.click(await screen.findByRole("button", { name: "Check Allotment" }));

    expect(await screen.findByText("Bigshare Services")).toBeVisible();
    expect(screen.queryByLabelText("Provider mode")).not.toBeInTheDocument();
    expect(screen.getByText("Verification required")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Check All Accounts" }));

    expect(await screen.findByRole("heading", { name: "Example IPO" })).toBeVisible();
    expect(screen.getByText("Partially complete · 1 of 2 final")).toBeVisible();
    expect(screen.getByText("Allotted")).toBeVisible();
    expect(screen.getByText("Verification Required")).toBeVisible();
    expect(screen.getByRole("button", { name: "Continue Verification for Friend" })).toBeVisible();
    expect(screen.getByRole("link", { name: "Open Official Page" })).toHaveAttribute(
      "href",
      "https://ipo.bigshareonline.com/ipo_status.html",
    );
    expect(screen.queryByText("ABCDE1234F")).not.toBeInTheDocument();
  });
});
