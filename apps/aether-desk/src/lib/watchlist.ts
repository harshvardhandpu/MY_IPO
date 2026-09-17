import { create } from "zustand";
import { persist } from "zustand/middleware";

type WatchState = {
  slugs: string[];
  toggle: (slug: string) => void;
  has: (slug: string) => boolean;
};

export const useWatchlist = create<WatchState>()(
  persist(
    (set, get) => ({
      slugs: ["ss-retail", "jindal-supreme", "lcc-projects", "rentomojo"],
      toggle: (slug) =>
        set((s) => ({
          slugs: s.slugs.includes(slug)
            ? s.slugs.filter((x) => x !== slug)
            : [...s.slugs, slug],
        })),
      has: (slug) => get().slugs.includes(slug),
    }),
    { name: "aether-ipo-watch" },
  ),
);
