# Profit Calculation

Financial math is deterministic and performed per allocation using integer paise. Percentages use basis points (`1000` = 10%).

## Realized calculation

```text
gross_proceeds = sale_price_per_share × sold_shares
acquisition_cost = allotted_cost + configured_charges
gross_profit = gross_proceeds - acquisition_cost
friend_share = eligible ? max(gross_profit, 0) × share_bps / 10000 : 0
member_net_profit = gross_profit - friend_share
```

Rounding uses integer division with an explicit half-up policy at the final share calculation. Negative profit never creates a negative friend share.

## Estimated profit

```text
estimated_profit = allotted_shares × (reference_price - issue_price)
```

A result must include reference price, source/basis, and timestamp. If unavailable, the UI displays `Not available yet`; allotment alone never implies profit.

## Aggregation

Allocation → IPO/member → member → group. Never calculate a friend's share against an entire member or IPO total.

## Required cases

- profitable eligible friend at 10%;
- profitable ineligible friend;
- primary account;
- zero and negative profit;
- custom share rate;
- multiple allocations with different eligibility;
- unavailable estimate basis;
- values near integer/rounding boundaries.
