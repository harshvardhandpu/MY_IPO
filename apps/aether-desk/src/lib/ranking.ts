import { getIpo, gmpPct, minInvest } from "@/lib/ipo-data";

export type Recommendation = {
  label: string;
  skip: boolean;
  reason: string;
  recommendedAccounts: number;
  lots: number;
  amountEach: number;
  confidence: "LOW" | "MEDIUM" | "HIGH";
};

export function previewRecommendation(input: {
  ipoSlug: string;
  capital: number;
  selectedAccounts: number;
  lots: number;
}): Recommendation {
  const ipo = getIpo(input.ipoSlug);
  if (!ipo) {
    return {
      label: "Issue unavailable",
      skip: true,
      reason: "Typed name is not in the public catalogue.",
      recommendedAccounts: 0,
      lots: 0,
      amountEach: 0,
      confidence: "LOW",
    };
  }
  const each = minInvest(ipo) * Math.max(1, input.lots);
  const maxAccounts = Math.max(1, Math.floor(input.capital / each) || 1);
  const recommended = Math.min(input.selectedAccounts, maxAccounts);
  const pct = gmpPct(ipo);
  const hot = ipo.subscription.total >= 2;
  const skip = pct <= 0 && ipo.subscription.retail >= 4;
  const confidence = pct >= 15 && hot ? "HIGH" : pct >= 5 ? "MEDIUM" : "LOW";
  const reason = skip
    ? `Public book is cold (GMP ${pct.toFixed(1)}%, retail ${ipo.subscription.retail.toFixed(2)}x). Ranking suggests skip.`
    : `Public GMP ${pct.toFixed(1)}% · total book ${ipo.subscription.total.toFixed(2)}x · ${ipo.board}. Uses only catalogue fields.`;
  return {
    label: "Placeholder ranking until owner algorithm exists",
    skip,
    reason,
    recommendedAccounts: skip ? 0 : recommended,
    lots: input.lots,
    amountEach: each,
    confidence,
  };
}
