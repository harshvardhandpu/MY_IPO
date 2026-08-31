# Product: Historical application entry

## Problem
The owner has a genuine IPO application from before Sanket IPO existed. The current Invest screen is phrased for new applications and cannot truthfully distinguish a historical owner-entered fact from a newly submitted plan.

## Success metric
One owner can record one historical application with its real amount, optional application date, existing account, registrar, and public issue mapping while creating zero automated allotment results and zero real provider requests.

## Announcement — the blog post before the feature
Sanket IPO can now record applications that predate the app without pretending they were created at the original investment time. Owners enter the known facts, leave unknown optional facts blank, and explicitly affirm that the record is historical. The application is linked to the existing private account while PAN remains inside the encrypted identity boundary. Provider results stay unchecked until the owner separately authorizes a live lookup.

## Screens
- `mockups/historical-application.html` — owner-controlled form for a historical IPO application and explicit provenance affirmation.
