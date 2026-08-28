# MUFG Intime India (ex Link Intime) — IPO allotment status research

**Date tested:** 2026-08-28  
**Live PAN used:** none

## Official entry points

| Item | Value |
|---|---|
| Investor hub | https://in.mpms.mufg.com |
| Public issues / application status | https://in.mpms.mufg.com/Initial_Offer/public-issues.html |
| Rights issues check | https://web.in.mpms.mufg.com/rightsoffers/rightsissues-chkApp.html |
| Helpdesk | ipo.helpdesk@in.mpms.mufg.com / (0) 810 811 4949 |

## Observed workflow (public)

1. Open Public Issues application status page.
2. Select company/issue from dropdown.
3. CAPTCHA image present (`CImage.aspx`) on the public-issues page — **human verification likely required**.
4. Submit application lookup and read status.

## Automation characteristics

| Question | Finding |
|---|---|
| CAPTCHA on public status page? | **Yes** (image captcha observed) |
| Stable API without human step? | **Unlikely** while captcha is mandatory |
| Browser required? | **Yes** for captcha path |
| PAN used? | Expected for IPO application status |

## Sanket strategy

- Provider id `mufg_intime`.
- Default live path: **NEEDS_HUMAN_VERIFICATION** when captcha is required.
- Fixture adapter for CI.
- Do **not** use captcha-solving services.
- Domain allowlist: `*.mufg.com`, `in.mpms.mufg.com`, `web.in.mpms.mufg.com`.

## Fallback

Continue Verification UX + open official page + manual result.
