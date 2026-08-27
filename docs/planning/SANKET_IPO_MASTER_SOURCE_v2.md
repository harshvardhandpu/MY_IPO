# Sanket IPO — Master Project Source of Truth

**Status:** Planning / architecture baseline — **v2.0 (Quick Invest + Automated Allotment milestone integrated)**  
**Created from:** Complete project explanation given on 27 August 2026 plus the follow-up investment/allotment requirements given on 27–28 August 2026  
**Purpose:** This file is the authoritative project source. It preserves the user's requirements and then adds implementation architecture without silently removing or changing those requirements.

## v2.0 Change Summary

This revision integrates the complete follow-up conversation about PAN capture, the Dashboard **Invest** flow, algorithm-assisted investment checking, registrar/RTA discovery, background allotment checking, PAN-scoped automation, and allotment report cards. It also adds the implementation-agent model fallback order and the current application repository.

**Current application repository:** `https://github.com/harshvardhandpu/MY_IPO.git`

**Important privacy correction:** PAN is now mandatory for core members and friend accounts, but full PAN values are treated as encrypted sensitive identity data. They must never be exposed to external AI models, logs, Git commit messages, filenames, analytics, or plaintext Obsidian notes.

---

# 0. Non-Negotiable Project Constraints

1. **Native desktop application, not a website.**
2. **Windows and Linux must both be supported.**
3. **Only approved core members may access the application and the private GitHub repositories.**
4. **The owner has an additional privileged/admin role.**
5. **All core members can see the other core members' IPO activity, investments, profits, friend-account activity, and proofs as intended by the group.**
6. **AI models must have zero access to private core-member data, friend data, UPI IDs, payment screenshots, member profits, or other personal/financial records.**
7. **Two separate Obsidian knowledge/data areas are required:**
   - Member/private operational data.
   - IPO intelligence, research, news, strategies, bot experiments.
8. **GitHub private repositories are the shared synchronization backbone.**
9. **The app must automatically synchronize data among members.**
10. **Installation must be close to one-command / one-script setup.**
11. **Development and operation should be zero-cost wherever realistically possible.**
12. **Security should be practical and strong without creating a slow, enterprise-heavy system that takes months to implement.**
13. **The app should feel fast and should not make members wait for unnecessary security or synchronization steps.**
14. **The system must retain timestamps and an audit trail for important changes.**
15. **No requirement in this source should be silently discarded. Architectural substitutions are marked as such.**

16. **PAN is mandatory for every core member and every friend/external account that may be used for IPO applications.**
17. **Full PAN values are sensitive identity data and must be encrypted at rest and in Git-synchronized storage.**
18. **External AI models are never allowed to receive PAN values. Automated allotment checking that requires PAN must run through a narrow local non-AI automation capability.**
19. **The Dashboard must expose two first-class quick actions: `Invest` and `Check Allotment`.**
20. **Allotment checking must run in the background without stealing the user's active browser/tab focus whenever the registrar permits automation.**
21. **Registrar/RTA integrations must be provider-based rather than hard-coded to a single website.**
22. **CAPTCHAs or equivalent anti-bot challenges must never be bypassed. If a registrar requires human verification, the job pauses safely and requests that verification.**
23. **The algorithm-ranking bot may receive only a sanitized investment-decision payload derived from the Invest form; private identity/payment fields remain outside the AI boundary.**


---

# 1. Product Vision

Sanket IPO is a private, desktop-first IPO group operating system for a small trusted group of Indian IPO investors.

It is not intended to be a public SaaS website in its first version. It should feel like a high-quality SaaS product while running as a native application on Windows and Linux.

The application combines:

- group IPO investment tracking;
- core-member accounting;
- primary and friend-account investment tracking;
- proof screenshots;
- allotment tracking;
- friend profit-sharing;
- profit calculations;
- visual dashboards;
- group-wide analytics;
- Indian IPO news;
- IPO ranking/research reports;
- multi-provider AI research;
- a separate experimental strategy-learning / paper-trading system;
- Git-backed shared Obsidian knowledge;
- optional future database providers such as Supabase.

---

# 2. User / Role Model

## 2.1 Owner

The owner is the highest-privilege user.

Owner responsibilities and permissions:

- manage the application's core-member allowlist;
- approve or remove core members;
- manage AI providers;
- add/edit/remove AI API base URLs;
- add/edit/remove model names;
- manage API credentials;
- configure AI provider order and failover;
- configure IPO ranking algorithms;
- configure bot research / evaluation rules;
- view all group and member information;
- manage application-level settings;
- manage repositories/vault configuration;
- initiate or approve schema migrations;
- manage security recovery;
- manage registrar/RTA provider configuration used by allotment checking;
- inspect allotment-job health without viewing PAN in logs;
- configure the investment-ranking algorithm version used by the Dashboard `Check` action.

The owner has a separate privileged/admin interface.

## 2.2 Core Member

A core member is an actual member of the IPO group.

A core member can:

- log into the application;
- maintain their own primary investment account;
- add any number of friend/external accounts they use for IPO applications;
- archive/remove friend accounts from future use;
- mark selected friends as eligible for a profit share;
- create IPO investment entries;
- add investment allocations from their own account and friend accounts;
- upload payment proofs;
- record allotments;
- record realized returns;
- record payments of profit share to eligible friends;
- upload proof of the profit-share payment;
- view their own dashboard;
- view the other core members' dashboards and investment records;
- view group-level totals and analytics;
- use News, Report, and permitted AI research functions;
- view bot-training / paper-trading results;
- use the Dashboard `Invest` quick action;
- use the Dashboard `Check Allotment` quick action for eligible IPOs;
- trigger account-by-account background allotment checks for IPOs they/group have submitted.

## 2.3 Friend / External Account

A friend is **not** a core member and does not automatically receive application access.

A friend account is a record used by a core member for IPO application tracking.

Possible fields:

- friend/account ID;
- display name;
- account-holder name;
- relationship/note (optional);
- broker/trading platform (optional, e.g. Angel One or other platform);
- UPI ID used for IPO mandates/payment;
- **PAN (mandatory; stored encrypted; masked in normal UI);**
- PAN consent/authorization acknowledgement and timestamp where applicable;
- optional masked bank/account metadata;
- active/archived state;
- default profit-share eligibility;
- default profit-share percentage (initial default requirement: 10%);
- created timestamp;
- archived timestamp;
- owning core-member ID.

A core member may add **as many friend accounts as required**.

---

# 3. Navigation / Main Application Areas

The original request described a three-dot navigation control. The product should preserve that concept while still providing a fast desktop UX.

Primary navigation:

1. **Dashboard**
2. **Members**
3. **Investments**
4. **News**
5. **Report**
6. **Bot Training**
7. **Notifications / Activity**
8. **Settings**
9. **Owner/Admin** — owner only

The three-dot menu can be the compact entry point, while desktop layouts may additionally show a sidebar for speed.

The Dashboard itself also has two persistent high-priority quick actions:

- **Invest** — opens the daily/multi-IPO investment form.
- **Check Allotment** — opens pending/eligible IPOs and launches the background allotment workflow.

---

# 4. Dashboard — Landing Page

The landing page is a high-quality visual dashboard.

## 4.1 Group Summary

Show:

- total money invested by the entire group;
- total amount from members' primary accounts;
- total amount invested through friend accounts;
- total realized profit;
- total pending/unrealized profit where applicable;
- total friend profit-share amount paid;
- total net group profit after friend profit shares;
- number of active IPO applications;
- number of allotted IPOs;
- number of non-allotted IPO applications;
- allocation / success rate;
- current open investments.


## 4.1A Dashboard Quick Actions

### Invest

A clearly visible `Invest` button appears in the upper area of the landing dashboard for every authorized core member.

It opens an **Investment Session / Daily Invest** form that supports:

- declared total capital the member intends to deploy that day;
- one or more IPO rows;
- manually typed full IPO name;
- `+ Add IPO` beside the IPO-name area;
- manually entered planned amount/price **per account** for each IPO;
- the member's own primary account included by default;
- the member's active friend accounts listed below;
- `+`/selection controls for adding multiple friend accounts;
- two bottom actions: **Check** and **Submit**.

The product requirement explicitly says the typed IPO name should not be blocked merely because the application cannot validate the spelling. The app may suggest matches, but manual input remains possible.

### Check Allotment

A clearly visible `Check Allotment` button appears beside/near `Invest`.

It displays IPOs for which:

- an investment has already been submitted;
- at least one account is still in `PENDING_ALLOTMENT`;
- the recorded/estimated allotment date has arrived **or** the registrar provider reports that the issue is available for status lookup.

The member selects an IPO and starts the account-by-account allotment job.

## 4.2 Top Performing IPOs

Show top IPOs that produced the highest:

- absolute profit;
- return percentage;
- listing gain;
- net profit after friend profit share.

Cards may include company logo/image if a legitimate source is available.

Clicking an IPO opens its IPO Detail page.

## 4.3 Current IPO Investments

Show current IPOs in which the group has applied.

Each IPO card should show:

- IPO/company name;
- optional company image/logo;
- application status;
- total group amount applied;
- number of core members participating;
- number of external/friend accounts used;
- allotment status if known.

Clicking the IPO opens a dedicated page showing:

- every core member who participated;
- amount invested by each;
- own-account amount;
- friend-account amount;
- all relevant friend-account allocations;
- payment proofs;
- allotment details;
- profit calculations when available.

## 4.4 Core Member Performance List

As the user scrolls down, display core members.

For every core member show:

- member name;
- total invested;
- total profit;
- net profit;
- number of IPOs invested in;
- number of IPO allotments;
- number of friend accounts used;
- friend-share payouts;
- return percentage.

Clicking a member opens that member's dashboard.

## 4.5 Sorting

Standard sorting/filtering must be available.

Examples:

- profit high → low;
- profit low → high;
- investment high → low;
- investment low → high;
- return % high → low;
- name A → Z;
- name Z → A;
- most IPOs;
- most recent activity;
- most allotments;
- date range;
- IPO status;
- member.

---

# 5. Member Dashboard

Every core member has a drill-down dashboard visible to other core members.

Show:

- total invested;
- primary-account invested amount;
- friend-account invested amount;
- total realized profit;
- gross profit;
- friend-share deductions;
- net profit;
- count of IPOs participated in;
- count of allotments;
- list of IPO companies;
- active friend accounts;
- archived friend accounts;
- number of friend payouts;
- graphs of investment and profit over time.

Clicking a company/IPO from a member dashboard opens the member-specific view of that IPO.

---

# 6. Core Member Onboarding

When a core member account is created, collect the minimum necessary information.

Possible onboarding fields:

- display name;
- email;
- app username/member ID;
- optional mobile number;
- primary broker/trading account/platform;
- primary UPI ID;
- **PAN (mandatory);**
- PAN confirmation/consent acknowledgement;
- primary account label;
- local authentication credential.

Do not collect unnecessary sensitive data simply because it might be useful later.

### PAN Handling During Onboarding

- PAN is required before the member can be used as an IPO application account.
- Validate the basic PAN format locally before acceptance.
- Show only a masked form such as `ABCDE****F` in ordinary UI.
- Encrypt the full PAN before persistence.
- Never place the full PAN in plaintext Markdown, JSON event payloads, Git commit messages, filenames, crash reports, telemetry, or AI prompts.

After onboarding, the member can add friend accounts.

---

# 7. Friend Account Management

## 7.1 Add Friend Account

A member can add unlimited friend/external accounts.

For each:

- name / account label;
- UPI ID;
- **PAN (mandatory for any friend account that can be selected for IPO investment);**
- PAN consent/authorization acknowledgement;
- broker/platform;
- optional notes;
- profit-share eligible: yes/no;
- profit-share percentage when eligible;
- default is 10% when the member chooses the original 10% rule.

Not every friend receives 10%.

Eligibility must be explicitly configurable per friend.

## 7.2 Remove / Archive Friend

A member can remove a friend from future investment use.

**Architectural rule:** historical financial records must never be physically deleted just because a friend is removed.

Instead:

- mark the friend account as archived/inactive;
- retain historical investments;
- retain allotments;
- retain proofs;
- retain payouts;
- retain timestamps;
- retain audit events.

When a friend account is archived/removed:

- every core member receives an in-app notification;
- activity feed records which core member archived which friend;
- timestamp is recorded automatically.

---

# 8. IPO Investment Creation Flow

When a member participates in an IPO, the app must not assume that all money came from one source.

Create an IPO application/investment record.

## 8.1 IPO Details

Store:

- IPO ID;
- company name;
- issue type where relevant;
- issue open date;
- issue close date;
- price band;
- lot size;
- application date/time;
- member ID;
- status.

## 8.2 Primary Account Allocation

Always ask separately:

- amount invested from the member's own/primary account;
- UPI ID used;
- number of lots / shares if known;
- payment/mandate proof screenshot;
- optional notes.

## 8.3 Friend Account Allocations

For every friend account used:

- friend account;
- amount applied/invested;
- UPI ID used;
- lots/shares if known;
- payment proof screenshot;
- profit-share eligibility for this IPO;
- profit-share percentage for this IPO;
- notes.

An investment may have:

- primary account only;
- one friend account;
- many friend accounts;
- primary + many friend accounts.

## 8.4 Investment Session / Dashboard `Invest` Form

The new Dashboard `Invest` flow is a higher-level workflow that can create multiple IPO application records in one session.

### Step A — Declare Daily Capital

Ask:

- `Total amount planned for investment today`.

This is a planning/control figure. It does **not** replace the account-level accounting records.

### Step B — Add IPO Rows

Each IPO row contains:

- manually entered full IPO name;
- planned investment amount/price per selected account;
- optional notes;
- optional registrar if already known.

A `+` control adds another IPO row.

Manual names are accepted. Search/autocomplete can be offered as a convenience, but inability to find an exact match must not automatically reject the entry.

### Step C — Select Accounts

The core member's own primary account is selected/included by default.

Below it show every active friend account owned by that member.

The member may select any number of friend accounts using a `+`/toggle control.

For decision analysis, the system can derive:

- number of available accounts;
- number of selected accounts;
- per-IPO amount per account;
- implied total capital if the same amount is applied to each selected account.

### Step D — `Check`

`Check` is **decision preview**, not final financial submission.

Before calling an external AI model, construct a sanitized `InvestmentDecisionPayload`.

Allowed fields include:

- IPO names typed in this form;
- per-IPO planned amount per account;
- declared daily capital;
- count of selected application accounts;
- anonymous account slots such as `ACCOUNT_1`, `ACCOUNT_2`, etc.;
- public IPO data collected specifically for the entered IPOs;
- the owner-provided/versioned algorithm.

Forbidden fields include:

- PAN;
- UPI ID;
- member name;
- friend name;
- email/mobile;
- proof screenshots;
- historical private member profits/investments unless the owner later creates an explicitly safe aggregate feature;
- MemberVault paths/content.

The manually configured algorithm-ranking bot then:

1. researches/loads only the public IPO information required by the algorithm;
2. scores/ranks the IPOs entered in the form;
3. recommends an investment ratio;
4. recommends how many application accounts should be assigned to each IPO;
5. may recommend avoiding a poor candidate completely;
6. may recommend concentrating all available selected accounts on one IPO if the algorithm strongly prefers it;
7. returns a short reason below each recommendation;
8. identifies missing/low-confidence public data rather than inventing it.

The response is a recommendation preview. It never submits an IPO application and never changes financial records by itself.

### Step E — Apply Recommendation

The UI may provide an `Apply Recommendation` control that maps the suggested account counts/ratios back onto the form.

The human member remains the final decision maker and may modify the result.

### Step F — `Submit`

`Submit` persists the member's actual chosen plan.

It creates:

- one investment session;
- IPO application records;
- account allocations;
- relevant audit events;
- expected allotment metadata when available;
- background synchronization work.

If the user clicks `Submit` without `Check`, submission must still work. AI availability must never block accounting.

## 8.5 Arithmetic Validation

The app should catch arithmetic mistakes without second-guessing the member's IPO name.

Examples:

- negative/zero amount where invalid;
- selected accounts = 0;
- derived allocation total exceeds the declared daily capital;
- duplicate account selection;
- duplicate IPO row warning.

Warnings should be clear and fast. The user can correct them before Submit.

## 8.6 Registrar and Expected Allotment Metadata at Submission

After or around submission, the public-intelligence/provider layer should attempt to determine:

- registrar/RTA;
- official allotment-status provider;
- expected/final allotment date if publicly available;
- current status-check availability.

This lookup uses public IPO metadata only. No PAN is needed for registrar/date discovery.

If the data is unknown, store `UNKNOWN` and retry later rather than inventing a date.


---

# 9. Proof / Screenshot Requirements

The user explicitly requires proofs.

Proof types include:

1. primary IPO payment/application proof;
2. friend-account IPO payment/application proof;
3. allotment evidence when manually uploaded;
4. friend profit-share payout proof;
5. optional sale/realization proof.

Every proof stores:

- proof ID;
- related entity;
- encrypted blob/file;
- original filename;
- file hash;
- uploaded-by member;
- timestamp;
- optional note.

Proofs must not be exposed to any external AI model.

---

# 10. Automatic Date and Time Tracking

The app automatically records timestamps for important events.

At minimum:

- record created;
- record updated;
- IPO application recorded;
- proof uploaded;
- allotment recorded;
- sale/return recorded;
- friend share calculated;
- friend share marked paid;
- friend payment proof uploaded;
- friend archived;
- member added;
- member removed;
- algorithm changed;
- strategy changed;
- AI provider changed;
- sync events.

Use UTC internally and display local time in the app.

---

# 11. Allotment Flow

When IPO allotment results are known, a member records the result separately for every account used.

Possible states:

- pending;
- allotted;
- not allotted;
- partially allotted where relevant;
- cancelled/rejected.

For an allotted account, record:

- account allocation source;
- allotted lots;
- allotted shares;
- allotment amount;
- allotment timestamp/entry timestamp;
- optional proof.

This must work independently for:

- member primary account;
- every friend account.

## 11.1 Automated Background Allotment Checking

Manual entry remains supported, but the preferred new workflow is automated account-by-account checking.

### Eligibility

`Check Allotment` lists submitted IPOs whose accounts remain pending and whose status page is likely available.

### Job Creation

When a member selects an IPO:

1. resolve the IPO's registrar/RTA provider;
2. load every application allocation for that IPO;
3. resolve the account holder for each allocation;
4. obtain the encrypted PAN reference for each account;
5. create one `AllotmentCheckJob`;
6. process each account sequentially or with very conservative provider-safe concurrency.

### PAN Capability Boundary

The allotment checker is **not an LLM/AI prompt**.

It is a local deterministic provider worker with a narrow `PAN_LOOKUP` capability.

Only that worker may temporarily decrypt PAN for a lookup.

External AI models, the ranking bot, News AI, Strategy Lab, and Report AI remain unable to request or receive PAN.

### Background Operation

The user should be able to continue working while the job runs.

Preferred implementation order:

1. stable official/public machine-readable endpoint **if legitimately available and permitted**;
2. deterministic HTTP/form adapter where technically stable;
3. isolated headless browser automation worker when a browser DOM is required;
4. human-assisted step only when the registrar requires CAPTCHA/OTP/other explicit human verification.

The worker must not commandeer the user's normal browser, change their active tab, or move focus during ordinary checks.

### CAPTCHA / Anti-Bot Rule

Do **not** attempt to bypass CAPTCHA, OTP, bot protection, rate limits, or other security controls.

If encountered:

- mark the account/job `NEEDS_HUMAN_VERIFICATION`;
- show a compact in-app action;
- open an isolated verification window only when the member chooses to continue;
- resume the job after legitimate completion.

### Per-Account Result

Store:

- check result ID;
- IPO ID;
- member/friend account ID;
- provider ID;
- check timestamp;
- `ALLOTTED`, `NOT_ALLOTTED`, `PENDING`, `NOT_FOUND`, `NEEDS_HUMAN_VERIFICATION`, `PROVIDER_ERROR`, or `UNKNOWN`;
- allotted quantity/lots if returned;
- application number/demat reference if already stored and safe to show;
- sanitized provider response metadata;
- optional encrypted evidence screenshot/hash;
- retry count;
- next retry timestamp when appropriate.

Do not persist PAN into the result object.

## 11.2 Registrar/RTA Provider Abstraction

Do not hard-code KFintech logic across the application.

Create an interface similar to:

```text
AllotmentProvider
├── provider_id()
├── supports(ipo)
├── discover_issue()
├── availability()
├── supported_lookup_keys()
├── check_allotment(lookup_context)
├── requires_human_verification()
└── normalize_result()
```

Initial provider targets should include adapters for the major official registrars encountered by the group's IPOs.

The user's explicitly requested first target is **KFin Technologies (KFintech)**.

Other adapters can include providers such as **Bigshare Services** and **MUFG Intime India** as required by actual IPO registrar assignments.

Provider web interfaces can change, so selector/form details belong in isolated adapters and contract tests rather than in domain logic.

## 11.3 Provider Discovery

Every IPO record should contain:

- `registrar_id`;
- `registrar_name`;
- official allotment-status URL when known;
- provider capability status;
- last verified timestamp.

If registrar metadata cannot be resolved automatically, allow the member/owner to select it manually.

## 11.4 KFintech First Adapter

The KFintech adapter should support the official IPO-status flow for issues registered there.

Design goals:

- choose the correct IPO/issue;
- use PAN only inside the local allotment worker;
- parse and normalize the returned status;
- never log PAN;
- capture provider failures cleanly;
- rate-limit requests;
- retain no unnecessary browser/session state after the job completes.

The integration must be tested against current website behavior before each production release because public HTML/forms can change.

## 11.5 Allotment Report Card

After all accounts for an IPO are processed, generate an `AllotmentReportCard`.

The report card shows a row/card per application account:

- core member or friend display name;
- account type (`PRIMARY` / `FRIEND`);
- masked PAN only;
- allotment status;
- allotted shares/lots if known;
- application amount;
- estimated profit when a valid price basis exists;
- profit-estimate basis;
- friend profit-share eligibility where relevant;
- last checked timestamp;
- provider/source;
- `View Evidence` where an encrypted result proof exists.

### Estimated Profit Rule

Allotment status by itself is not enough to know profit.

Estimated profit must state its basis, for example:

- actual market/listing price after listing;
- owner-selected expected listing price;
- another explicitly configured public estimate.

Formula example:

`estimated_profit = allotted_shares × (reference_price − issue_price)`

If no valid reference price exists, display:

`Estimated profit: Not available yet`

Never fabricate an estimated profit simply because an account was allotted.

## 11.6 Automatic Follow-Up

For pending jobs:

- retry only at reasonable intervals;
- stop retrying when finalized;
- avoid hammering registrar websites;
- notify the member when all account results are final;
- update the IPO/member/group dashboard using normalized allotment results.

## 11.7 Manual Fallback

If a provider becomes incompatible or automation is blocked:

- preserve the job;
- allow `Open Official Status Page`;
- allow manual result entry;
- flag the entry as `MANUAL`;
- retain audit provenance.

The entire application must not fail because one registrar adapter is temporarily broken.


---

# 12. Profit Calculation Model

Profit must be calculated at the smallest meaningful allocation level, not only at the member total.

For each allotted allocation:

**Gross proceeds**
= sale value or chosen valuation basis.

**Acquisition/application cost**
= allotted investment amount plus configured charges if the group wants them included.

**Gross profit**
= gross proceeds − acquisition/application cost − configured charges.

If the friend is profit-share eligible:

**Friend profit share**
= max(gross profit, 0) × configured share percentage.

Initial/common share:
- 10%.

But it can be 0% or another configured value if the owner later allows it.

**Net member profit**
= gross profit − friend profit share.

Then aggregate:

allocation → IPO/member → member total → group total.

The app must clearly show the calculation behind displayed profit.

---

# 13. Friend Profit-Share Settlement

If a friend is eligible for a 10% (or configured) share:

Show:

- gross attributable profit;
- share percentage;
- amount due;
- payment status;
- paid date;
- proof screenshot.

Payment status:

- not due;
- due;
- paid;
- waived/adjusted with note if permitted.

A member must be able to upload proof that they paid the friend.

---

# 14. Investment Page

The Investment page is the analytical view of group activity.

## 14.1 Group Visualizations

Show:

- total invested over time;
- total profit over time;
- primary vs friend funding;
- IPO-wise capital allocation;
- member-wise allocation;
- member-wise profit;
- allotted vs non-allotted;
- friend-share deductions;
- net profit.

## 14.2 Member Table

Columns may include:

- member;
- total invested;
- own-account amount;
- friend-account amount;
- number of IPOs;
- allotments;
- gross profit;
- friend payouts;
- net profit;
- ROI.

## 14.3 Company / IPO Table

Columns may include:

- IPO;
- total amount applied;
- members participating;
- external accounts used;
- allotted accounts;
- gross return;
- net group profit.

Clicking an IPO opens the IPO Detail page.

---

# 15. IPO Detail Page

This is one of the most important drill-down pages.

Show:

- company / IPO header;
- timeline;
- total group investment;
- group primary-account investment;
- group friend-account investment;
- members who participated;
- account-level allocations;
- allotments;
- proofs;
- sale / profit information;
- friend-share calculations;
- payout proofs;
- graphs.

For every participating core member, show:

- member amount;
- own-account amount;
- friend-account amount;
- friend list used for this IPO;
- amount from each friend account;
- account allotment result;
- profit per account;
- 10% share eligibility;
- amount paid to eligible friend;
- proof buttons.

---

# 16. News Page

The News page focuses primarily on **Indian IPOs**.

It should also include global events that may materially influence Indian IPO markets or market sentiment.

Examples:

- major geopolitical conflict;
- war;
- interest-rate events;
- macroeconomic shocks;
- large market crashes/rallies;
- regulatory changes;
- sector-specific events;
- major commodity/FX events where relevant.

## 16.1 News UI

Each story can contain:

- title;
- source;
- publication timestamp;
- related IPO/company;
- relevance category;
- short summary;
- image;
- video embed when legitimately available;
- source link;
- impact classification.

The page should have image/video-rich cards where available.

## 16.2 AI Boundary

News/public intelligence can be supplied to AI models.

Private member information cannot.

The AI may receive:

- public news;
- IPO names;
- public company information;
- offer documents;
- public historical IPO data;
- owner-provided algorithms;
- research papers;
- strategy notes from the intelligence vault.

The AI may **not** receive:

- member identity details;
- member email;
- friend details;
- UPI IDs;
- screenshots;
- member investment amounts;
- member/group profit data;
- private proofs;
- member vault contents.

---

# 17. Report Page

Report is the IPO research / recommendation workspace.

It has at least two main modes.

## 17.1 Algorithm Ranking Mode

Members enter or select a list of IPOs.

The system applies the owner-provided algorithm and mathematical criteria.

The AI model is used to:

- obtain/structure permitted public data;
- apply the algorithm;
- explain the scoring;
- compare the IPOs;
- rank them.

The algorithm, not the AI's unsupported opinion, is authoritative in this mode.

Output:

- ranked IPO list;
- score per IPO;
- pass/fail per criterion;
- key reasons;
- risk factors;
- missing-data warning;
- source references.

## 17.2 Suggest Mode

A separate AI-assisted mode may propose its own candidate strategy/ranking based on the permitted intelligence corpus.

It can study:

- Indian IPO history;
- current IPO data;
- public news;
- market conditions;
- research papers;
- prior strategy results.

This mode must clearly display that it is experimental and probabilistic.

---

# 18. AI Provider System

The owner wants multiple AI APIs because individual providers/models can exhaust limits.

Therefore create a provider-independent AI gateway.

Each provider configuration:

- provider ID;
- display name;
- base URL;
- API protocol type;
- API key/secret reference;
- models available;
- enabled/disabled;
- priority;
- usage status;
- cooldown;
- last error;
- optional rate/token limits.

## 18.1 Failover

When one model/provider becomes unavailable or exhausted:

1. try another endpoint for the preferred model if configured;
2. otherwise choose the next compatible model;
3. preserve the task's required capability;
4. record failover in an internal technical log.

## 18.2 Owner-Only Provider Management

Only the owner may:

- add keys;
- edit keys;
- remove keys;
- change provider order;
- enable/disable models.

Core members can use enabled research features but do not need to manually configure providers.

---

# 19. Important Secret-Key Architecture Correction

Original idea:
> Put API keys in a text file committed to the private GitHub repository so members do not need to enter them.

**Do not store plaintext API keys in Git.**

Even a private repository has multiple risks:

- every collaborator can read the keys;
- keys remain in Git history after deletion;
- compromised collaborator credentials expose them;
- accidental repository exposure leaks them.

## Selected replacement

Preserve the zero-manual-setup experience, but store secrets encrypted.

Recommended design:

- encrypted owner-managed `secrets.bundle.age` or equivalent encrypted secret bundle;
- device/member public-key recipients;
- local decrypted secret copied into the OS credential store;
- Windows: Windows Credential Manager;
- Linux: Secret Service/keyring where available;
- app never writes plaintext secrets back to Git;
- logs automatically redact API keys.

Important limitation:
If a core member's device must call a third-party API directly using a shared API key, a sufficiently privileged user on that device can ultimately extract the credential. The only way to completely hide a shared secret from clients is a trusted server/proxy, which conflicts with the current zero-cost/no-server preference. Therefore the project assumes core members are trusted but protects secrets from accidental exposure and external compromise.

---

# 20. AI Data Isolation — Hard Security Boundary

The strongest project security rule is:

> The AI subsystem must be technically unable to read the member vault.

This must not rely only on prompting.

Architecture:

- separate repository;
- separate directory;
- separate storage adapter;
- separate application service;
- explicit allowlisted file roots;
- no AI tool/API receives member-vault paths;
- AI request builder accepts only `IntelligenceContext`;
- private-domain data types cannot be serialized into the AI request path;
- automated tests attempt to inject UPI/member/proof data and must fail.

AI service sees:

`IPO Intelligence Vault + public sources + owner algorithms`

AI service does not see:

`Member Vault + proofs + UPI + PAN + member/friend identity records + member financial records`

### Three Separate 'Bot' Concepts Must Not Be Blurred

1. **Algorithm Ranking Bot** — may use an external AI model, but only receives sanitized Invest-form/public IPO data plus the owner-provided algorithm.
2. **Allotment Automation Worker** — deterministic local automation; has narrowly scoped temporary PAN access; is **not** allowed to send PAN to an LLM.
3. **Strategy Lab** — experimental public-data research/backtesting system; has no MemberVault/PAN access.

This separation is mandatory in code, permissions, tests, and documentation.

---

# 21. Bot Training Page

The user requested a separate three-dot navigation section for the experimental IPO bot.

Name options:

- Bot Training
- Strategy Lab
- IPO Lab

Recommended UI name: **Strategy Lab**, with "Bot Training" as the familiar subtitle.

---

# 22. What "Training Every 10 Minutes" Means

With external API models, the app normally cannot literally retrain the underlying model every ten minutes.

Therefore the requirement is implemented as a **continuous strategy research/evaluation loop**, not hidden fine-tuning.

Every cycle can:

1. collect new permitted public information;
2. deduplicate it;
3. classify IPO relevance;
4. update the intelligence vault;
5. retrieve historical analogues;
6. ask an available model to propose/modify strategy rules;
7. backtest strategy candidates;
8. reject weak strategies;
9. record metrics;
10. promote the best validated strategy to a candidate pool.

Default requested interval:
- every 10 minutes while enabled and when AI quota/model availability allows.

When all AI providers are unavailable:
- continue deterministic data processing;
- queue AI-dependent research;
- resume after a provider becomes available.

---

# 23. Intelligence / Training Corpus

The separate IPO intelligence vault should include public information such as:

- Indian IPO data from 2021 onward;
- IPO offer dates;
- issue price;
- lot size;
- issue size;
- subscription;
- listing price;
- listing gain/loss;
- later performance checkpoints if the strategy uses them;
- public company financials;
- DRHP;
- RHP;
- prospectus;
- public exchange/regulator material;
- sector information;
- market indices;
- relevant news;
- macro events;
- research papers;
- strategy hypotheses;
- strategy versions;
- backtest results;
- failed strategies;
- data-source quality notes.

The research history must remain reproducible.

---

# 24. Strategy Validation Target

The user's desired outcome is an extremely high success ratio, approximately 95–99%, especially for identifying IPO opportunities capable of very high returns such as >70%.

This target is preserved as a **research aspiration**, not a promised application capability.

The system must never fake or manipulate results to reach the target.

Required safeguards:

- no look-ahead bias;
- no future-news leakage;
- train/research on earlier periods and evaluate on later unseen periods;
- walk-forward validation;
- out-of-sample evaluation;
- enough sample size before claiming success;
- confidence intervals;
- calibration;
- separate metrics for:
  - allotment prediction;
  - listing gain;
  - >0% return;
  - >20% return;
  - >50% return;
  - >70% return;
- failed predictions are retained permanently.

A displayed 95–99% metric is allowed only if the exact metric, sample size, period, and validation method are shown.

---

# 25. Paper Trading / Simulation

The bot gets a virtual capital allocation.

Requested starting simulation capital:
- approximately **₹60,000**.

The strategy engine may simulate multiple imaginary application accounts/friends.

Simulation can include a configurable assumption such as:
- "at least one simulated account receives an allotment"

This assumption must be visibly labeled because it does not represent guaranteed real-world IPO allotment probability.

The bot records:

- IPO considered;
- date;
- strategy version;
- score;
- virtual amount applied;
- imaginary accounts used;
- simulated allotment;
- listing/sale assumptions;
- resulting return;
- profit/loss;
- cumulative balance.

No real trade should be executed by this module in the initial system.

---

# 26. Bot Training / Strategy Lab Dashboard

Show:

- virtual starting capital;
- current virtual balance;
- total simulated profit;
- total simulated return;
- win rate;
- loss rate;
- IPOs evaluated;
- simulated IPOs selected;
- >70% return hit rate;
- average return;
- median return;
- maximum drawdown;
- strategy version;
- model/provider used;
- data freshness;
- latest research cycle;
- historical performance chart;
- individual simulated trades/investments.

Every strategy change must be versioned.

---

# 27. Obsidian / Vault Architecture

The user wants Obsidian to act as durable human-readable memory/storage.

Use two primary vaults.

## 27.1 Private Member Vault

Suggested repository:
`SanketIPO-PrivateData`

Vault:
`MemberVault/`

Contains:

- core members;
- friend accounts;
- IPO applications;
- allocation events;
- allotments;
- profit records;
- friend-share calculations;
- payout records;
- activity/audit events;
- notifications;
- references to proofs;
- encrypted proof files.

AI access:
**DENIED**

### Sensitive Identity Subsystem

PAN must not be stored as readable frontmatter/plaintext inside member or friend Markdown files.

Recommended representation:

```text
MemberVault/
└── _secure_identity/
    ├── <member-id>.enc
    └── friends/
        └── <friend-id>.enc
```

The normal profile stores only:

- an opaque sensitive-record ID;
- masked PAN;
- verification/consent metadata.

The encrypted sensitive record contains the full PAN.

Approved application devices obtain the group decryption material through the secure secret/device-enrollment mechanism and load it into local secure memory only when required.

The background allotment worker requests PAN through a narrow method such as:

`SensitiveIdentityService.with_pan(account_id, purpose=ALLOTMENT_CHECK)`

The method must:

- authorize the current user/device;
- decrypt only the requested identity;
- return it only to the local provider worker;
- redact logs;
- zero/drop temporary buffers where practical;
- create an audit event stating that a sensitive lookup occurred **without recording the PAN value**.



## 27.2 IPO Intelligence Vault

Suggested repository:
`SanketIPO-Intelligence`

Vault:
`IntelligenceVault/`

Contains:

- IPO public data;
- news;
- filings;
- research;
- source metadata;
- ranking algorithms;
- strategy research;
- paper-trading events;
- bot evaluation reports;
- prompt templates;
- model outputs that are safe to persist.

AI access:
**ALLOWED**

## 27.3 Application Source

Suggested third private repository:
`SanketIPO-App`

Contains:

- desktop application source;
- installer;
- updater;
- schemas;
- tests;
- documentation;
- migrations.

This three-repository split is more secure and maintainable than putting code, private financial data, screenshots, AI research, and secrets into one Git repository.

---

# 28. GitHub Sync Model

Original requirement:
- every member constantly pushes/pulls;
- ideally every second;
- no one should be left behind.

Preserved product goal:
**near-real-time shared state with no manual Git operations.**

Implementation must not do a Git commit/push every second.

## 28.1 Local-First Save

Whenever a member makes a change:

1. validate input;
2. write locally immediately;
3. update the UI immediately;
4. append an event to the local event log;
5. queue a background sync.

User does not wait for network sync to complete.

## 28.2 Debounced Push

Suggested initial policy:

- synchronize a few seconds after the last change;
- batch related edits;
- never create hundreds/thousands of meaningless Git commits per hour.

Example:
- local save: immediate;
- commit queue debounce: 3–10 seconds;
- push as soon as batch is ready and repository is clean;
- periodic pull/fetch: approximately 15–30 seconds while app is active;
- pull on application startup;
- pull on application focus;
- pull before important financial writes;
- manual "Sync Now" button.

Intervals are configurable.

## 28.3 Offline Operation

If internet is unavailable:

- save locally;
- mark changes `Pending Sync`;
- continue normal use;
- automatically synchronize when connectivity returns.

## 28.4 Sync Status

Always show:

- Synced;
- Syncing;
- Pending;
- Offline;
- Conflict needs attention;
- Auth required.

Do not make the user open a terminal for normal sync issues.

---

# 29. Git Conflict Prevention — Event-Sourced Design

Do not have all laptops constantly edit the same giant JSON/Markdown/SQLite file.

Instead use immutable or append-only domain events.

Example:

`MemberVault/Events/<member-id>/<device-id>/<date>/<event-id>.json`

Each important action creates a uniquely named event file.

Examples:

- `MEMBER_CREATED`
- `FRIEND_ADDED`
- `FRIEND_ARCHIVED`
- `IPO_APPLICATION_CREATED`
- `ALLOCATION_ADDED`
- `PROOF_ATTACHED`
- `ALLOTMENT_RECORDED`
- `PROFIT_RECORDED`
- `FRIEND_SHARE_DUE`
- `FRIEND_SHARE_PAID`

Benefits:

- different devices normally create different files;
- Git merges are much safer;
- complete audit history;
- accidental overwrites are reduced;
- local dashboards can be rebuilt from events;
- corrupted derived data can be regenerated.

---

# 30. Local Database for Speed

Obsidian/Git remains the durable source.

For application speed, each laptop has a local generated SQLite database.

Important rule:

> SQLite is a local materialized cache/index, not the shared Git source.

Never repeatedly commit the live SQLite database across multiple machines.

Flow:

`Git/Obsidian events → local indexer → SQLite → fast UI`

When Git receives new events:

- index only new events;
- update SQLite;
- refresh affected dashboard widgets.

This gives fast SaaS-like performance while retaining the Git/Obsidian source.

---

# 31. Proof File Storage

Screenshots are binary and can make Git history grow quickly.

Use:

- compression before storage;
- normalized JPEG/WebP/PNG depending on proof type;
- cryptographic hash;
- immutable file name;
- content-addressed path.

Example:

`MemberVault/_secure_blobs/ab/cd/<sha256>.bin`

Recommended:
- encrypt private proof files before committing.

The local app decrypts them only when an authorized member clicks **View Proof**.

Do not let Obsidian plugins, AI indexing, or preview generation automatically send proof data externally.

---

# 32. Git LFS Policy

Git LFS is optional, not a first-day requirement.

Reason:
screenshots may eventually make normal Git repositories large.

However strict zero-cost operation means storage/bandwidth must be monitored.

Initial plan:

- compress images aggressively;
- keep proof files reasonably small;
- monitor repository growth;
- move to Git LFS only if needed;
- configure billing to stop rather than charge if a free quota is exhausted.

---

# 33. Authentication and Access

Absolute "cannot be hacked" security cannot be guaranteed.

The target is **strong practical security with low friction**.

## 33.1 Access Layers

Layer 1 — GitHub private repository membership  
Only authorized GitHub accounts can fetch the application/private data repositories.

Layer 2 — Local app authentication  
After installation, user signs into/unlocks the desktop app.

Layer 3 — Role manifest  
The owner-signed membership configuration determines:
- owner;
- active core members;
- revoked members.

Layer 4 — encrypted local/private data and secrets.

## 33.2 Fast Login

Recommended UX:

First setup:
- GitHub authentication;
- membership verification;
- create local PIN/password;
- optional OS credential storage.

Normal launch:
- quick PIN/unlock;
- optionally remember unlocked state for a reasonable period on trusted devices.

No heavy MFA prompt every time unless the owner later enables it.

---

# 34. Membership / Device Revocation

When a core member leaves:

- remove GitHub repository access;
- revoke member ID;
- revoke device IDs;
- rotate shared encrypted secrets;
- prevent future sync.

Important reality:
A person who previously had legitimate local access may still possess old copies of previously downloaded data. GitHub revocation cannot erase files already copied to a former member's machine.

The app should minimize this risk but cannot technically guarantee remote deletion of historical clones.

---

# 35. Native Desktop Technology

Recommended primary stack:

## Desktop Shell
**Tauri 2**

Reasons:

- Windows support;
- Linux support;
- native desktop packaging;
- small footprint relative to bundling a full browser runtime;
- Rust core for privileged local filesystem/Git/security operations;
- capability/permission model;
- signed update support;
- open source.

## Frontend
- React
- TypeScript
- Vite

## UI
- Tailwind CSS
- shadcn/ui or equivalent open-source component system
- Recharts / Apache ECharts for visualization

## Native Core
- Rust / Tauri commands

## Local Database
- SQLite

## Git Operations
Prefer controlled native Git operations:
- `git2`/libgit2 where appropriate; or
- carefully wrapped system Git initially for faster development.

## Schema / Validation
- TypeScript Zod on UI/input boundary;
- Rust serde validation on native boundary.

## Allotment Automation Runtime

Keep registrar automation behind a replaceable runtime abstraction.

Preferred engineering order:

1. HTTP/client adapter when an official public workflow is stable and lawful to automate;
2. headless-browser worker for sites that require DOM interaction;
3. human-assisted verification for CAPTCHA/OTP.

A Playwright-compatible headless worker is a practical first prototype because it supports Windows/Linux and can run without taking over the user's browser, but the implementation should remain replaceable so a lighter Rust-native adapter can be used later.

Do not make the main UI depend directly on CSS selectors from registrar websites.


---

# 36. Tauri Permission Model

The webview/frontend should not receive arbitrary filesystem or shell access.

Use narrowly scoped Tauri capabilities.

Examples:

- frontend may request `create_investment`;
- Rust validates data and writes only to the member vault;
- report frontend may request `run_public_ipo_analysis`;
- AI service can read only IntelligenceVault;
- AI commands do not contain a member-vault filesystem permission.

This makes the AI/private-data separation structural.

---

# 37. One-Command Installation

Requirement:
Each approved core member should be able to install from private GitHub with a single script.

Because Windows and Linux use different shells, provide two first-class bootstrap commands while preserving the one-script experience on each OS.

Repository:

`bootstrap/install.ps1` — Windows  
`bootstrap/install.sh` — Linux

The installer should:

1. verify OS and architecture;
2. verify required Git/GitHub authentication;
3. authenticate the approved GitHub account if needed;
4. verify member allowlist access;
5. download the latest signed application release;
6. install the app;
7. create application directories;
8. clone/fetch MemberVault;
9. clone/fetch IntelligenceVault;
10. generate a device ID;
11. register/enroll the device;
12. configure local SQLite;
13. configure secure credential storage;
14. fetch/decrypt permitted AI provider configuration;
15. perform integrity checks;
16. create desktop/start-menu application entry;
17. start the application.

Normal members should not manually:
- install Node;
- install Rust;
- run `npm install`;
- build the app.

Those are developer activities.

---

# 38. Application Updates

Preferred production flow:

1. owner publishes signed version;
2. members' apps check for a new version;
3. signed package is downloaded;
4. signature is verified;
5. update is installed;
6. data vaults are never deleted by application update.

Tauri supports signed update artifacts for Windows and Linux.

Because the code repository is private, the release/update download path must preserve GitHub authentication or use an approved private release download mechanism.

Fallback:
- installer/updater uses GitHub CLI authenticated as the core member to fetch private release assets.

---

# 39. Zero-Cost Development Rule

The project should default to:

- open-source frameworks;
- GitHub Free;
- private repositories;
- local computation;
- local SQLite;
- local Obsidian vaults;
- free model/provider quotas supplied by the owner;
- official/free public information sources;
- no required paid database;
- no required paid server;
- no required paid auth SaaS.

## 39.1 Guardrails Against Surprise Cost

- do not attach a payment method where avoidable;
- configure provider hard usage caps;
- disable paid AI fallback;
- set GitHub usage budgets/limits;
- prefer local scheduled jobs over cloud cron;
- make Supabase optional;
- make external services replaceable.

"Zero cost" means the app should stop/degrade gracefully when a free quota ends rather than silently incur charges.

---

# 40. Optional Supabase / Database Integration

The core v1 does not require Supabase.

Create a storage abstraction.

Example interfaces:

- `MemberRepository`
- `InvestmentRepository`
- `ProofRepository`
- `IntelligenceRepository`
- `SyncProvider`

Implementations:

V1:
- `GitVaultProvider`
- `LocalSQLiteIndex`

Future:
- `SupabaseProvider`
- other database/API provider.

MCP can be used by development/agent tooling if useful, but the desktop app itself should use a proper application/database API rather than treating MCP as the fundamental database transport.

AI/MCP tools must not be granted access to MemberVault.

---

# 41. Notifications / Activity Feed

Required notification events include:

- core member added;
- core member removed;
- friend account added if desired;
- friend account archived/removed;
- IPO investment created;
- allotment recorded;
- payout marked complete;
- sync conflict;
- important news;
- report completed;
- strategy milestone.

Original explicit requirement:
when a core member removes/archive a friend account, every other core member must receive a message.

---

# 42. News / Research Data Sources

Source priority should be:

1. official regulator/exchange/company filings;
2. reliable financial/news sources;
3. secondary aggregators;
4. AI-generated interpretation only after source collection.

For Indian IPO filings, official SEBI public-issue filings should be treated as a high-authority source.

All intelligence records should retain:

- source URL;
- publication date;
- retrieval timestamp;
- source name;
- content hash;
- IPO/company tags;
- confidence / source quality.

---

# 43. AI Research Prompt/Data Pipeline

Safe pipeline:

`Public sources`
→ `collector`
→ `cleaner`
→ `deduplicator`
→ `source metadata`
→ `IntelligenceVault`
→ `retriever`
→ `algorithm engine`
→ `AI reasoning/explanation`
→ `validation`
→ `report`

There is **no edge** from `MemberVault` to `AI Gateway`.

---

# 44. Ranking Algorithm Architecture

Do not embed one ranking formula directly into the UI.

Store versioned algorithms.

Example:

`IntelligenceVault/Algorithms/ipo-ranking/v001.yaml`

Algorithm definition may include:

- criteria;
- weight;
- minimum threshold;
- normalization;
- hard reject rules;
- missing-data handling;
- scoring formula;
- version;
- author;
- created date;
- change log.

Every report saves:
- algorithm version used;
- data snapshot/date;
- AI model/provider;
- score;
- result.

This ensures past decisions remain reproducible when the algorithm changes.

---

# 45. Strategy Engine Architecture

Strategies are versioned similarly.

Lifecycle:

`DRAFT`
→ `BACKTESTING`
→ `VALIDATION`
→ `PAPER_TRADING`
→ `CANDIDATE`
→ `REJECTED` or `PROMOTED`

Never overwrite a strategy's past result after changing its rules.

Create a new version.

---

# 46. No Fake Learning

The Strategy Lab must not simply ask an LLM "improve the strategy" repeatedly and declare success.

Each improvement must be measured.

Possible loop:

1. hypothesis;
2. proposed factor;
3. data availability check;
4. historical backtest;
5. leakage test;
6. validation set test;
7. robustness test;
8. compare against baseline;
9. retain/reject;
10. record rationale.

Only objectively better candidates advance.

---

# 47. Auditability

Financial records need a complete audit trail.

Every event records:

- event ID;
- event type;
- actor/member;
- device ID;
- entity ID;
- timestamp;
- previous entity revision;
- payload;
- content hash;
- app version.

For high-value events, optionally sign events with a per-device cryptographic key.

The app can then flag:
- modified history;
- invalid events;
- duplicate events.

---

# 48. Data Integrity

Do not rely on Git commit history alone as financial integrity.

Use:

- stable IDs;
- schema version;
- event hash;
- immutable events;
- validation;
- local rebuild;
- periodic snapshot/checkpoint;
- encrypted backup.

If one member corrupts their local checkout, they should be able to restore from the private remote + snapshots.

---

# 49. Proposed Member-Vault Structure

```text
MemberVault/
├── README.md
├── config/
│   ├── group.yaml
│   └── schema-version
├── members/
│   └── <member-id>/
│       └── profile.md
├── friends/
│   └── <member-id>/
│       └── <friend-id>.md
├── ipos/
│   └── <ipo-id>/
│       └── metadata.md
├── events/
│   └── <member-id>/
│       └── <device-id>/
│           └── 2026/
│               └── 08/
│                   └── <event-id>.json
├── snapshots/
├── notifications/
├── _secure_blobs/
│   └── <hash-prefix>/
└── audit/
```

---

# 50. Proposed Intelligence-Vault Structure

```text
IntelligenceVault/
├── 00-Inbox/
├── 01-IPOs/
│   └── <ipo-id>/
│       ├── profile.md
│       ├── market-data/
│       ├── filings/
│       └── news/
├── 02-News/
├── 03-Filings/
│   ├── DRHP/
│   ├── RHP/
│   └── Prospectus/
├── 04-Research/
├── 05-Algorithms/
├── 06-Strategies/
├── 07-Backtests/
├── 08-Paper-Trading/
├── 09-Model-Research/
├── 10-Reports/
├── 11-Prompts/
├── 12-Sources/
└── 99-Archive/
```

---

# 51. Suggested App Source Structure

```text
SanketIPO-App/
├── apps/
│   └── desktop/
│       ├── src/                    # React/TypeScript
│       └── src-tauri/              # Rust/Tauri
├── crates/
│   ├── domain/
│   ├── member-vault/
│   ├── intelligence-vault/
│   ├── sync-engine/
│   ├── crypto/
│   ├── ai-gateway/
│   ├── strategy-engine/
│   ├── calculation-engine/
│   └── audit/
├── packages/
│   ├── ui/
│   ├── schemas/
│   └── charts/
├── bootstrap/
│   ├── install.ps1
│   └── install.sh
├── scripts/
├── migrations/
├── docs/
├── tests/
└── .github/
```

---

# 52. Core Domain Objects

Main objects:

- `Owner`
- `CoreMember`
- `Device`
- `FriendAccount`
- `IPO`
- `IPOApplication`
- `InvestmentAllocation`
- `Proof`
- `Allotment`
- `Sale/Realization`
- `ProfitCalculation`
- `FriendProfitShare`
- `FriendSharePayment`
- `Notification`
- `AuditEvent`
- `NewsItem`
- `PublicSource`
- `Algorithm`
- `AlgorithmRun`
- `AIProvider`
- `AIModel`
- `AIResearchTask`
- `Strategy`
- `Backtest`
- `PaperPortfolio`
- `PaperApplication`
- `PaperResult`
- `SensitiveIdentityRecord`
- `InvestmentSession`
- `InvestmentDraft`
- `InvestmentDecisionPayload`
- `InvestmentRecommendation`
- `RegistrarProvider`
- `RegistrarIssueMapping`
- `AllotmentCheckJob`
- `AllotmentCheckAttempt`
- `AllotmentCheckResult`
- `AllotmentReportCard`

---

# 53. Financial Calculation Hierarchy

```text
Group
└── Core Member
    └── IPO
        ├── Primary Account Allocation
        │   └── Allotment → Profit
        └── Friend Account Allocation(s)
            └── Allotment
                ├── Gross Profit
                ├── Friend Profit Share
                └── Net Member Profit
```

This hierarchy is essential to prevent incorrect 10% deductions.

---

# 54. Synchronization Architecture

```text
               Private GitHub
          ┌──────────┴──────────┐
          │                     │
   Member Data Repo       Intelligence Repo
          │                     │
   ┌──────┴──────┐        ┌─────┴─────┐
   │ Sync Engine │        │ Sync Engine│
   └──────┬──────┘        └─────┬─────┘
          │                     │
   Member Vault           Intelligence Vault
          │                     │
  Local SQLite Index      Local Search Index
          │                     │
          └──── Desktop App ────┘
                    │
             AI Gateway
                    │
          Intelligence only
```

---

# 55. AI Isolation Diagram

```text
                 ┌────────────────────┐
                 │ Desktop UI         │
                 └───────┬────────────┘
                         │
            ┌────────────┴─────────────┐
            │                          │
    Private Domain Service      Intelligence Service
            │                          │
     MemberVault + SQLite       IntelligenceVault
            │                          │
            │                       AI Gateway
            │                          │
            │                    External Models
            │
            X  NO CONNECTION TO AI
```

---

# 56. Fast-Path UX

A normal investment entry should require minimal steps.

Example:

1. choose IPO;
2. own amount;
3. choose friend accounts;
4. enter amount per friend;
5. attach proofs;
6. Save.

App immediately displays `Saved locally`.

Background:
- event creation;
- encryption;
- local SQLite update;
- Git sync.

The member should not wait for Git before continuing.

---

# 57. MVP Scope — Build This First

## Phase 1 — Foundation

- Tauri Windows/Linux shell;
- application navigation;
- local profile/login;
- owner/core-member roles;
- MemberVault;
- event model;
- SQLite index;
- Git sync engine;
- sync status.

## Phase 2 — Identity + IPO Accounting

- mandatory encrypted PAN for core members;
- mandatory encrypted PAN for investable friend accounts;
- sensitive identity service;
- members;
- friend accounts;
- add/archive;
- IPOs;
- investment allocations;
- proof upload;
- allotment;
- profit calculation;
- 10% friend share;
- payout proof;
- timestamps;
- audit;
- Dashboard `Invest` form;
- multi-IPO investment sessions;
- `Check` recommendation preview;
- `Submit` persistence flow;
- registrar metadata discovery;
- KFintech-first provider adapter;
- background `Check Allotment` jobs;
- normalized allotment report cards.

## Phase 3 — Dashboards

- group dashboard;
- member dashboard;
- investment page;
- IPO drill-down;
- sorting/filtering;
- charts;
- notifications.

## Phase 4 — News

- public Indian IPO sources;
- news ingestion;
- images/source cards;
- relevance classification;
- intelligence vault.

## Phase 5 — Report

- owner algorithm editor/import;
- ranking engine;
- AI gateway;
- multiple AI providers;
- failover;
- ranking chatbot;
- Suggest mode.

## Phase 6 — Strategy Lab

- historical corpus;
- research loop;
- backtesting;
- ₹60k paper portfolio;
- imaginary accounts;
- metrics;
- strategy versioning;
- 10-minute research schedule.

## Phase 7 — Packaging

- Windows release;
- Linux release;
- one-command installers;
- authenticated private update flow;
- recovery tooling.

---

# 58. Features That Should NOT Delay MVP

Do not initially spend weeks on:

- cloud Kubernetes;
- microservices;
- paid auth provider;
- blockchain;
- custom end-to-end messenger;
- custom database server;
- complex zero-knowledge cryptography;
- real brokerage trade execution;
- automatic real-money IPO application;
- true model fine-tuning;
- mobile app;
- public multi-tenant SaaS;
- enterprise SIEM;
- biometric-only auth.

These can be revisited if the product grows.

---

# 59. Security MVP

Implement these from day one:

- private repositories;
- GitHub access control;
- local app lock;
- password/PIN hashing;
- encrypted secrets;
- encrypted proof blobs;
- AI/private-data hard isolation;
- least-privilege Tauri capabilities;
- safe file-path handling;
- sanitized logs;
- API key redaction;
- signed releases;
- schema validation;
- dependency scanning;
- immutable audit events;
- backup/recovery;
- no plaintext secrets in Git.

Additional PAN/allotment controls:

- no plaintext PAN in Git;
- no plaintext PAN in Obsidian notes;
- no PAN in filenames/commit messages;
- no PAN in crash reports or telemetry;
- no PAN in AI prompts;
- masked PAN in normal UI;
- full PAN reveal disabled by default;
- encrypted identity envelopes;
- purpose-limited PAN decryption for allotment checks;
- registrar-request rate limiting;
- browser/session data destroyed after sensitive jobs;
- CAPTCHA/OTP bypass prohibited;
- account-holder consent/authorization acknowledgement;
- dependency isolation between AI gateway and sensitive identity service.


This provides strong practical security without enterprise overengineering.

---

# 60. Performance Requirements

The app should:

- open quickly;
- render dashboard from local SQLite instead of querying Git/network;
- save changes instantly to local storage;
- sync in background;
- lazy-load proof images;
- lazy-load news media;
- index only changed events;
- avoid reprocessing the entire vault on every change.

Target UX:
most normal local actions should feel immediate.

---

# 61. Failure Handling

The application must remain usable when:

- GitHub is temporarily unavailable;
- internet is down;
- AI API quota is exhausted;
- one AI provider fails;
- news source fails;
- a sync conflict happens.

Private accounting functions must **never depend on AI availability**.

AI failure cannot block:
- investments;
- allotments;
- profit calculations;
- dashboards;
- proofs;
- member management.

---

# 62. Backups

At minimum:

- Git remote copy;
- local repository;
- periodic encrypted snapshot archive;
- local SQLite can be rebuilt.

Owner should have a simple:
**Backup Health** screen.

Show:
- last successful private-vault push;
- last intelligence-vault push;
- last local snapshot;
- last integrity check.

---

# 63. Privacy Rules

1. AI cannot access MemberVault.
2. News AI cannot access MemberVault.
3. Report AI cannot access MemberVault.
4. Strategy Lab cannot access MemberVault.
5. API logs cannot include member records.
6. External analytics/telemetry is OFF by default.
7. No private financial data is sent to third-party analytics.
8. Core members can see each other's data because this is an explicit group requirement.
9. Friends do not automatically receive application/repository access.
10. PAN is never exposed to any external AI model.
11. PAN may be decrypted only by authorized local application code for approved functions such as allotment lookup.
12. Ordinary UI uses masked PAN.
13. Allotment provider adapters receive only the minimum lookup identity required for that provider.
14. Sensitive lookup values are never written to application logs.

---

# 64. Owner Settings

Owner screen:

## Members
- invite/add;
- approve;
- revoke;
- devices.

## AI Providers
- provider;
- base URL;
- API credential;
- models;
- priority;
- enabled;
- quota state.

## Algorithms
- active version;
- edit/import;
- validate;
- history.

## Sync
- repositories;
- current branches;
- last sync;
- force sync;
- repair.

## Security
- secret rotation;
- membership keys;
- device revoke;
- backup;
- integrity.

## Strategy Lab
- enable/disable;
- schedule;
- virtual capital;
- simulation assumptions;
- target metrics.

---

# 65. Application Data Is Not the Same as Source Code

Do not mix all data into the source-code repository.

Recommended:

- `SanketIPO-App` — code/releases.
- `SanketIPO-PrivateData` — MemberVault.
- `SanketIPO-Intelligence` — IntelligenceVault.

All are private.

This keeps:

- code versioning clean;
- private data isolated;
- AI boundary enforceable;
- data recovery easier.

---

# 66. Git Branch / Sync Policy

Application source:
- normal development branches;
- protected main;
- releases from signed tags.

Operational MemberVault:
- a dedicated sync branch such as `live`;
- app-controlled commits;
- no members manually editing Git during normal use.

IntelligenceVault:
- bot/research commits can be separate from private financial operations;
- research does not block member sync.

---

# 67. Commit Policy

Do not use one commit for every low-level UI keystroke.

Create semantic commits.

Examples:

- `data: record IPO application <id>`
- `data: record allotment <id>`
- `data: archive friend <id>`
- `proof: attach payment proof <id>`
- `intel: ingest IPO news batch 2026-08-27T...`
- `strategy: record backtest v014`

The user still receives near-real-time data because the app saves locally immediately and background sync is frequent.

---

# 68. Research Loop Commit Policy

The AI/research loop should not create useless Git noise every ten minutes if nothing changed.

Commit only when:

- new source ingested;
- source changed;
- new strategy candidate created;
- backtest completed;
- strategy state changed;
- paper result recorded;
- report generated.

No-op cycles create no commit.

---

# 69. Source Provenance

Every AI/report claim should be traceable.

A report should show:

- input IPOs;
- algorithm version;
- source list;
- source timestamps;
- data freshness;
- model;
- provider;
- generation timestamp;
- score explanation.

This is more important than making the chatbot sound confident.

---

# 70. News Media Rules

For news cards:

- store thumbnail URL/metadata where possible;
- do not duplicate every external video into Git;
- embed/link legitimate video sources;
- cache only what is necessary;
- fall back to a text card if media unavailable.

This keeps the intelligence repository small.

---

# 71. Profitability / Decision-Support Principle

The app should help the group understand:

- where capital is being deployed;
- which members/accounts are performing;
- which IPOs are historically profitable;
- costs of friend profit shares;
- net rather than gross returns;
- allocation efficiency;
- strategy performance;
- whether a ranking approach actually beats a baseline.

It should not hide losses or unsuccessful applications.

---

# 72. Optional Future Productization

The architecture can later support:

- multiple independent groups;
- hosted sync;
- mobile app;
- subscription SaaS;
- managed AI;
- cloud database;
- automated brokerage integrations.

But **none of this is required for the internal zero-cost v1**.

---

# 73. Key Architectural Decisions

## Decision A
Use Tauri 2 + React/TypeScript for Windows/Linux desktop.

## Decision B
Use GitHub private repositories as zero-cost synchronization backbone.

## Decision C
Do not sync a live SQLite database through Git.

## Decision D
Use event files as durable operational truth and SQLite as local cache.

## Decision E
Keep MemberVault and IntelligenceVault separate.

## Decision F
No AI filesystem/tool permission for MemberVault.

## Decision G
Never put plaintext API keys in Git.

## Decision H
Use encrypted secret distribution + local OS credential store.

## Decision I
Interpret "AI training every 10 minutes" as strategy research/backtesting/evaluation, not unsupported model fine-tuning.

## Decision J
Treat 95–99% / >70%-return goal as a measurable research target, never a guaranteed claim.

## Decision K
Use paper trading only at first; no automated real-money trading.

## Decision L
Local actions are immediate; Git synchronization is asynchronous and debounced.

---

# 74. Acceptance Criteria for First Useful Version

A first genuinely useful release is complete when:

1. owner can install on Linux;
2. another member can install on Windows;
3. both authenticate;
4. both see the same group state;
5. a member can add a friend account;
6. other member sees the change after background sync;
7. member can create an IPO investment;
8. split it between own and multiple friend accounts;
9. upload proofs;
10. record independent allotments;
11. calculate gross profit;
12. calculate a selected friend's 10% share;
13. upload share-payment proof;
14. group dashboard updates;
15. member dashboard updates;
16. IPO detail shows full calculation;
17. audit timeline is correct;
18. removing a friend archives rather than destroys history;
19. all other members receive the removal notification;
20. AI subsystem tests prove that MemberVault cannot be read.

21. creating a core member without PAN is rejected;
22. creating an investable friend account without PAN is rejected;
23. the persisted Git/Obsidian data contains no plaintext PAN;
24. normal UI displays only masked PAN;
25. Dashboard `Invest` supports multiple IPO rows;
26. own account is included by default;
27. multiple friend accounts can be selected;
28. `Check` sends a sanitized payload and cannot contain PAN/UPI/name/proofs;
29. the algorithm bot returns ranking + account-count/allocation recommendations + short reasons;
30. `Submit` works even when AI is unavailable;
31. registrar and expected allotment metadata can be saved independently from PAN;
32. Dashboard `Check Allotment` shows pending/eligible invested IPOs;
33. selecting one IPO creates account-by-account allotment work;
34. a KFintech provider contract test can normalize a representative status response;
35. background checking does not take over the member's normal browser/tab;
36. provider CAPTCHA causes `NEEDS_HUMAN_VERIFICATION` rather than bypass;
37. no PAN appears in allotment-worker logs;
38. report card maps results back to member/friend names while showing only masked PAN;
39. estimated profit always shows its price basis or `Not available yet`;
40. a broken registrar adapter does not block manual allotment entry or the rest of the application.


Only after this should the project depend heavily on News/Report/Strategy Lab.

---

# 75. Recommended Development Order

**Build the accounting truth before the AI.**

Why:

If investment/allotment/profit data is wrong, a beautiful AI layer does not help.

Correct order:

`Data model + event schema`
→ `encrypted identity/PAN service`
→ `member/friend workflows`
→ `Dashboard Invest flow`
→ `algorithm Check payload + ranking engine`
→ `Submit/account allocations`
→ `registrar discovery + provider interface`
→ `KFintech allotment adapter`
→ `background allotment jobs + report card`
→ `profit + friend-share accounting`
→ `proofs`
→ `sync`
→ `dashboards`
→ `news`
→ `report AI`
→ `strategy lab`

---

# 76. Explicitly Preserved User Requirements Checklist

- [x] Native application.
- [x] Windows.
- [x] Linux.
- [x] Three-dot navigation concept.
- [x] Multiple pages.
- [x] Owner login.
- [x] Core-member accounts.
- [x] Members can see each other's dashboards/data.
- [x] Primary investment account.
- [x] Primary UPI ID.
- [x] Separate amount invested from own account for every IPO.
- [x] Unlimited friend/external accounts.
- [x] Friend UPI IDs.
- [x] Amount invested using each friend account.
- [x] Different trading/broker accounts supported.
- [x] Selected friends receive 10% profit share.
- [x] Not every friend must receive 10%.
- [x] IPO payment proof screenshot.
- [x] Friend-account payment proof screenshot.
- [x] Profit-share payment proof screenshot.
- [x] Automatic date/time.
- [x] Allotment records.
- [x] Profit calculation.
- [x] Dashboard with total investment.
- [x] Dashboard with total profit.
- [x] Top/high-return allotted IPOs.
- [x] Member list.
- [x] Member drill-down.
- [x] IPO drill-down.
- [x] Show calculations.
- [x] Group totals.
- [x] Sort minimum/maximum.
- [x] Alphabetical sorting.
- [x] Standard filters/sorting.
- [x] Investment visualizations.
- [x] Total number of IPOs per person.
- [x] Own vs friend investment visualization.
- [x] Add new core member.
- [x] Remove/archive friend account.
- [x] Notify every core member when a friend is removed.
- [x] Fast application.
- [x] News page.
- [x] Indian IPO focus.
- [x] Global events affecting IPOs.
- [x] News images/video.
- [x] Report page.
- [x] Owner-provided ranking algorithm.
- [x] AI applies algorithm and ranks selected IPOs.
- [x] Chat interface.
- [x] Separate Suggest mode.
- [x] Multiple AI providers.
- [x] Multiple base URLs.
- [x] Multiple API keys.
- [x] Provider/model failover.
- [x] AI has zero member/private financial access.
- [x] Historical IPO research from 2021 onward.
- [x] Continuous research.
- [x] 10-minute strategy/research loop.
- [x] Research papers.
- [x] News used for strategy research.
- [x] Separate intelligence Obsidian vault.
- [x] Separate member Obsidian vault.
- [x] Bot training/Strategy Lab page.
- [x] Approximately ₹60,000 virtual capital.
- [x] Imaginary friend accounts for simulation.
- [x] Configurable simulated allotment assumption.
- [x] Strategy improvement loop.
- [x] Desired 95–99% research-performance aspiration preserved.
- [x] Desired >70% high-return targeting preserved.
- [x] No false guarantee of those results.
- [x] Private GitHub repositories.
- [x] Git-backed sharing between laptops.
- [x] Automatic push/pull.
- [x] No member manually handling normal Git sync.
- [x] Near-real-time updates.
- [x] One-script/one-command installation.
- [x] Members do not manually enter AI APIs.
- [x] API secret distribution automated.
- [x] Plaintext API keys are not committed.
- [x] Optional future Supabase/database provider.
- [x] Zero-cost-first architecture.
- [x] Practical security without months of overengineering.

## v2.0 — Quick Invest + Allotment Requirements

- [x] PAN mandatory for every core member.
- [x] PAN mandatory for every investable friend/external account.
- [x] PAN encrypted rather than stored as plaintext in Obsidian/Git.
- [x] PAN masked in normal UI.
- [x] PAN excluded from all AI prompts/models.
- [x] Dashboard `Invest` quick-action button.
- [x] Dashboard `Check Allotment` quick-action button.
- [x] Daily total planned investment field.
- [x] Manual full IPO-name field.
- [x] `+ Add IPO` for multiple IPOs in one investment session.
- [x] Per-IPO manually entered planned amount/price per account.
- [x] Own primary account included by default.
- [x] Active friend list shown for selection.
- [x] `+`/toggle selection of multiple friend accounts.
- [x] `Check` and `Submit` as separate actions.
- [x] `Check` is a non-persistent recommendation preview.
- [x] `Check` sends only sanitized decision information to the algorithm bot.
- [x] Ranking bot works from owner-provided/versioned algorithm.
- [x] Ranking bot ranks only the IPOs entered/selected for the check.
- [x] Ranking bot recommends investment ratio.
- [x] Ranking bot recommends number of accounts per IPO.
- [x] Ranking bot may recommend avoiding a bad IPO.
- [x] Ranking bot may recommend concentrating all selected accounts on one IPO.
- [x] Ranking result includes a short reason.
- [x] Public-data retrieval may be used only for algorithm-required IPO facts.
- [x] `Submit` works even if no AI model is available.
- [x] Registrar/RTA discovered/stored per IPO.
- [x] Expected/final allotment date stored when publicly available.
- [x] Pending invested IPOs shown in `Check Allotment`.
- [x] User selects one IPO for the allotment run.
- [x] System gathers every application account used for that IPO.
- [x] PAN decrypted only inside narrow local allotment worker.
- [x] Allotment worker checks accounts one by one/provider-safely.
- [x] Background operation does not steal normal browser focus.
- [x] KFintech is the first explicitly requested allotment provider.
- [x] Provider interface supports additional RTAs.
- [x] Bigshare/MUFG Intime or other actual RTAs can be added as adapters.
- [x] CAPTCHA/OTP/security mechanisms are never bypassed.
- [x] Human verification state supported when registrar requires it.
- [x] Each account's normalized allotment result is persisted.
- [x] Allotment result maps back to friend/core-member identity.
- [x] Generated allotment report card.
- [x] Report card shows account-holder/friend name.
- [x] Report card shows masked PAN only.
- [x] Report card shows allotment status.
- [x] Report card shows allotted quantity/lots when available.
- [x] Report card shows estimated profit only with explicit price basis.
- [x] Provider failure supports retry/manual fallback.
- [x] Allotment automation is not given unrestricted MemberVault access.
- [x] Implementation-agent model fallback order is preserved.


---

# 77. Immediate Next Engineering Deliverables

The planning phase should next produce:

1. `REQUIREMENTS.md` derived from this source;
2. domain schema;
3. event schema;
4. vault schemas;
5. profit-calculation specification;
6. security threat model;
7. sync protocol;
8. UI screen map;
9. wireframes/design system;
10. Tauri project skeleton;
11. Windows installer;
12. Linux installer;
13. sensitive identity/PAN schema and encryption contract;
14. Dashboard Invest-form specification;
15. sanitized InvestmentDecisionPayload contract;
16. registrar/RTA provider interface;
17. KFintech provider spike/contract test;
18. background allotment job state machine;
19. allotment report-card schema;
20. first end-to-end test:
   `member/friend PAN → Invest session → Check recommendation → Submit → pending allotment → provider check → report card → profit → 10% share → proof → sync`.

This master source remains the authority when implementing those documents.

---


# 78.1 New Milestone — Quick Invest + Registrar Allotment Automation

This milestone is now part of the core application plan rather than an optional future feature.

## Product Flow

```text
Dashboard
   │
   ├── Invest
   │     │
   │     ├── Daily capital
   │     ├── IPO 1 ─ amount/account
   │     ├── IPO 2 ─ amount/account
   │     ├── ...
   │     ├── Own account [default]
   │     └── Friend accounts [+ select]
   │
   │         ┌──────────────┬──────────────┐
   │         │                             │
   │       Check                         Submit
   │         │                             │
   │  Sanitize payload             Persist real plan
   │         │                             │
   │  Algorithm Ranking Bot         Git/Event/SQLite
   │         │                             │
   │  Rank + account ratio          Pending Allotment
   │         │                             │
   │  Human decides                       │
   │                                       │
   └── Check Allotment ◄───────────────────┘
             │
       Select pending IPO
             │
       Resolve registrar
             │
       Account list
             │
      SensitiveIdentityService
             │
     PAN one account at a time
             │
       Local Provider Worker
             │
      KFin / other RTA
             │
      Normalize results
             │
      Allotment Report Card
             │
      Dashboard + profit engine
```

---

# 78.2 Investment Decision Payload Contract

The ranking bot should receive something shaped conceptually like:

```json
{
  "session_id": "opaque-id",
  "declared_daily_capital": 120000,
  "account_count": 6,
  "ipos": [
    {
      "typed_name": "Example IPO Limited",
      "planned_amount_per_account": 15000
    }
  ],
  "algorithm_version": "ipo-ranking-v001",
  "public_context_refs": []
}
```

It must **not** receive:

```text
PAN
UPI ID
member name
friend name
email
mobile
proof images
MemberVault paths
raw private financial history
```

If the algorithm requires "six accounts are available", it receives `account_count = 6`, not six people's identities.

---

# 78.3 Allotment Job State Machine

```text
CREATED
   ↓
WAITING_FOR_PROVIDER_AVAILABILITY
   ↓
RUNNING
   ├── account 1 → FINAL
   ├── account 2 → FINAL
   ├── account 3 → NEEDS_HUMAN_VERIFICATION
   └── account 4 → RETRYABLE_ERROR
   ↓
PARTIALLY_COMPLETE
   ↓
RUNNING / HUMAN_CONTINUATION / RETRY
   ↓
COMPLETE
```

Terminal per-account states:

- `ALLOTTED`
- `NOT_ALLOTTED`
- `NOT_FOUND`
- `MANUAL_RESULT`

Non-terminal/operational states:

- `PENDING`
- `NEEDS_HUMAN_VERIFICATION`
- `RATE_LIMITED`
- `PROVIDER_UNAVAILABLE`
- `RETRYABLE_ERROR`

---

# 78.4 Sensitive PAN Threat Model

## Assets

- full PAN;
- mapping from PAN → friend/member;
- UPI IDs;
- proofs;
- application/allotment history.

## Main Threats

1. plaintext PAN accidentally committed;
2. PAN printed in debug logs;
3. PAN sent to LLM in a generic "form context";
4. PAN leaked in browser screenshots/cache;
5. former member keeps previously cloned sensitive data;
6. malicious/compromised registrar page captures more data than necessary;
7. provider adapter breaks and returns wrong account mapping.

## Controls

- encrypted identity envelope;
- masked display;
- explicit safe DTOs between domains;
- no generic `serialize(member)` calls into AI;
- purpose-limited decryption;
- logging redaction tests;
- provider-domain allowlist;
- per-provider result normalization tests;
- signed/auditable events;
- device/member revocation;
- secret rotation when membership changes.

---

# 78.5 Allotment Provider Reliability Rules

Registrar websites are external dependencies and can change without notice.

Therefore:

- providers are plugins/adapters;
- every adapter declares supported lookup modes;
- selectors/endpoints stay inside the adapter;
- health checks determine whether the adapter is operational;
- a broken adapter is disabled automatically rather than returning guessed results;
- status parsing uses explicit known patterns;
- unknown output becomes `UNKNOWN`, never `NOT_ALLOTTED`;
- retries use backoff;
- requests are rate-limited;
- the official page can always be opened for manual fallback.

---

# 78.6 Current Registrar Research Notes

These are implementation research notes, not immutable product assumptions. Re-verify at coding/testing time.

- **KFin Technologies** currently exposes an official IPO allotment-status page and supports lookup using PAN among its available lookup choices.
- **Bigshare Services** currently exposes an IPO allotment page that includes PAN input and a CAPTCHA, therefore fully unattended automation may require a human-verification fallback.
- **MUFG Intime India** (formerly Link Intime) is another major registrar with public issue/status services and should be supported through a separate adapter when an IPO uses that registrar.

The application must never assume that all Indian IPOs use KFintech.

---

# 78.7 Repository / Workspace Baseline

Current application Git repository supplied for the project:

```text
https://github.com/harshvardhandpu/MY_IPO.git
```

Recommended local workspace, subject to verification during setup:

```text
~/HermesWorkspaces/MY_IPO
```

Inside the application repository, this master source should live at:

```text
docs/planning/SANKET_IPO_MASTER_SOURCE.md
```

Separate data/intelligence repositories remain recommended for production because PAN/member data and AI-readable research need different trust boundaries.

---

# 78.8 Hermes Implementation Model Routing

The implementation handoff should preserve the user's requested model priority.

```text
1. GPT-5.6
2. DeepSeek V4 Pro
3. Qwen 3.8
4. DeepSeek V4 Flash
```

Interpretation:

- GPT-5.6 is the primary architecture/implementation model.
- When its usable quota/tokens are exhausted, Hermes should continue from the existing repository state and handoff documents rather than restart the project.
- Fall back in the exact order above.
- The label `Qwen 3.8` is preserved exactly as supplied; during provider configuration Hermes should map it to the exact available provider/model identifier rather than guessing silently.
- Every model must read the same authoritative source and current handoff/state before writing code.
- Model switching must never change the product/security requirements.

---

# 78.9 Definition of Done for This Planning Increment

This new planning increment is considered fully integrated when the source includes:

1. mandatory encrypted PAN model;
2. Invest button/form;
3. multi-IPO rows;
4. own-account default;
5. multiple friend-account selection;
6. Check vs Submit semantics;
7. sanitized algorithm-bot payload;
8. account-count/ratio recommendations;
9. public IPO data retrieval boundary;
10. registrar discovery;
11. allotment-date metadata;
12. Check Allotment eligibility;
13. PAN-scoped local worker;
14. provider adapters;
15. KFintech-first implementation target;
16. background/non-focus-stealing execution;
17. CAPTCHA human fallback;
18. normalized per-account results;
19. report card;
20. estimated-profit basis rule;
21. AI/PAN hard isolation;
22. updated MVP/acceptance checklist;
23. Hermes model-fallback order.

All of the above are now part of this v2 source.

---

# 78. Final Architecture Summary

Sanket IPO should be built as a **local-first, Git-synchronized, private Windows/Linux desktop application**.

The app provides the user experience.

Obsidian-format vaults provide durable and inspectable knowledge/data.

GitHub private repositories provide free shared synchronization.

SQLite provides local speed.

Rust/Tauri provides the secure native boundary.

The MemberVault contains group financial operations plus encrypted sensitive identity references.

Full PAN values live only in the encrypted sensitive-identity subsystem and are exposed temporarily only to authorized local allotment-provider workers.

The IntelligenceVault contains public IPO/news/research/strategy information.

External AI models see only the IntelligenceVault plus explicitly sanitized Invest-form decision payloads. They never receive PAN, UPI IDs, proofs, member/friend identities, or raw MemberVault records.

Owner-managed encrypted secrets supply multiple AI providers.

The Strategy Lab continuously researches and backtests IPO strategies using public data and paper capital, while never pretending that an extremely high return or allotment success rate is guaranteed.

The Dashboard `Invest` and `Check Allotment` flows now sit directly on top of this architecture: members can plan multiple IPOs/accounts, request an algorithm-based ranking, submit the real allocation, and later run background registrar checks without giving sensitive identity data to AI.

This structure satisfies the core goal:

> A fast, private, zero-cost-first IPO group operating system that all approved members can install and use from Windows or Linux, while keeping financial data under the group's control and keeping AI isolated from private member information.
