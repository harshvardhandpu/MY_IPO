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
});
