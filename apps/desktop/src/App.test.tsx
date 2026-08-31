import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App, type CommandBridge } from "./App";

describe("desktop shell", () => {
  it("exposes the two primary dashboard actions", () => {
    render(<App />);

    const actions = screen.getByLabelText("Quick actions");
    expect(within(actions).getByRole("button", { name: "Invest" })).toBeVisible();
    expect(within(actions).getByRole("button", { name: "Check Allotment" })).toBeVisible();
  });

  it("shows local-first sync status without requiring a network call", () => {
    render(<App />);

    expect(screen.getByText("Saved locally")).toBeVisible();
    expect(screen.getByText("Pending sync")).toBeVisible();
  });

  it("renders only implemented navigation destinations and marks the active page", () => {
    render(<App />);

    const navigation = screen.getByRole("navigation", { name: "Primary navigation" });
    expect(navigation).toHaveTextContent("Dashboard");
    expect(navigation).toHaveTextContent("Members");
    expect(navigation).toHaveTextContent("Investments");
    expect(navigation).toHaveTextContent("Check Allotment");
    expect(navigation).not.toHaveTextContent("News");
    expect(navigation).not.toHaveTextContent("Reports");
    expect(screen.getByRole("button", { name: "Dashboard" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("renders dashboard activity from the existing safe candidate DTO", async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === "list_allotment_candidates") {
        return [
          {
            application_id: "app-1",
            session_id: "session-1",
            ipo_name: "Example IPO",
            planned_amount_paise: 1_482_000,
            account_count: 1,
            registrar_id: "mufg_intime",
            registrar_name: "MUFG Intime India",
            provider_id: "mufg-intime-live",
            provider_name: "MUFG Intime",
            provider_health: "READY",
            pending_count: 1,
            final_count: 0,
            last_checked: null,
            overall_job_state: "PENDING",
          },
        ];
      }
      if (command === "list_members") {
        return [{ id: "member-1", name: "Owner", role: "OWNER", masked_pan: "ABCDE****F" }];
      }
      if (command === "list_friends") return [];
      if (command === "get_dashboard") {
        return {
          total_planned_paise: 0,
          submitted_session_count: 0,
          member_count: 0,
          friend_count: 0,
          profit_paise: 0,
        };
      }
      throw new Error(`Unhandled command: ${command}`);
    });
    const bridge = { invoke } as CommandBridge;

    render(<App bridge={bridge} />);

    expect(await screen.findByText("Example IPO")).toBeVisible();
    expect(screen.getByText("MUFG Intime India")).toBeVisible();
    expect(screen.getByText("₹14,820")).toBeVisible();
  });
});
