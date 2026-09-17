import { create } from "zustand";
import { persist } from "zustand/middleware";

export const WIDGETS = [
  { id: "sentiment", label: "Market sentiment" },
  { id: "performance", label: "Book performance" },
  { id: "portfolio", label: "My book" },
  { id: "protect", label: "Protect capital" },
  { id: "heatmap", label: "Issue calendar" },
  { id: "quick", label: "Quick actions" },
  { id: "picks", label: "Top picks" },
  { id: "activity", label: "Last activity" },
] as const;

export type WidgetId = (typeof WIDGETS)[number]["id"];

type WidgetState = {
  hidden: WidgetId[];
  toggle: (id: WidgetId) => void;
  visible: (id: WidgetId) => boolean;
};

export const useWidgets = create<WidgetState>()(
  persist(
    (set, get) => ({
      hidden: [],
      toggle: (id) =>
        set((s) => ({
          hidden: s.hidden.includes(id) ? s.hidden.filter((x) => x !== id) : [...s.hidden, id],
        })),
      visible: (id) => !get().hidden.includes(id),
    }),
    { name: "aether-ipo-widgets" },
  ),
);
