import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("desktop shell", () => {
  it("exposes the two primary dashboard actions", () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "Invest" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Check Allotment" })).toBeVisible();
  });

  it("shows local-first sync status without requiring a network call", () => {
    render(<App />);

    expect(screen.getByText("Saved locally")).toBeVisible();
    expect(screen.getByText("Pending sync")).toBeVisible();
  });
});
