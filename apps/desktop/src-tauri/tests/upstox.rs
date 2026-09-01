use sanket_desktop_lib::upstox::{
    IpoStatus, PriceBasis, RegistrarMappingState, calculate_lot_plan, parse_details_response,
    parse_list_response,
};

const LIST_FIXTURE: &str = r#"
{
  "status": "success",
  "data": [{
    "id": "acme-ipo",
    "symbol": "ACME",
    "name": "Acme Industries Limited",
    "status": "open",
    "isin": "INE000000000",
    "issue_type": "regular",
    "issue_size": 123.45,
    "industry": "Manufacturing",
    "minimum_price": 100.00,
    "maximum_price": 120.50,
    "bidding_start_date": "2026-08-31",
    "bidding_end_date": "2026-09-02",
    "total_subscription": "3.25"
  }],
  "meta_data": { "page": 1 }
}
"#;

const DETAIL_FIXTURE: &str = r#"
{
  "status": "success",
  "data": {
    "id": "acme-ipo",
    "symbol": "ACME",
    "name": "Acme Industries Limited",
    "status": "open",
    "isin": "INE000000000",
    "issue_type": "regular",
    "issue_size": 123.45,
    "industry": "Manufacturing",
    "minimum_price": 100.00,
    "maximum_price": 120.50,
    "bidding_start_date": "2026-08-31",
    "bidding_end_date": "2026-09-02",
    "lot_size": 10,
    "minimum_quantity": 10,
    "cut_off_price": 120.50,
    "timeline": {
      "allotment_date": "2026-09-03",
      "listing_date": "2026-09-05"
    },
    "registrar_info": {
      "name": "KFin Technologies Limited",
      "registrar": "KFintech",
      "website": "https://kfintech.com"
    },
    "total_subscription": "3.25"
  }
}
"#;

#[test]
fn parses_list_without_float_authoritative_money_and_preserves_status() {
    let items = parse_list_response(LIST_FIXTURE, 1_756_633_600).expect("list fixture");
    let item = &items[0];
    assert_eq!(item.status, IpoStatus::Open);
    assert_eq!(item.minimum_price_paise, Some(10_000));
    assert_eq!(item.maximum_price_paise, Some(12_050));
    assert_eq!(item.bidding_start_date.as_deref(), Some("2026-08-31"));
    assert_eq!(item.source_ipo_id, "acme-ipo");
    assert_eq!(item.issue_size_crore.as_deref(), Some("123.45"));
    assert_eq!(item.industry.as_deref(), Some("Manufacturing"));
    assert_eq!(item.registrar_mapping_state, RegistrarMappingState::Unknown);
}

#[test]
fn details_choose_cutoff_and_derive_distinct_lot_math_and_registrar_alias() {
    let item = parse_details_response(DETAIL_FIXTURE, 1_756_633_600).expect("detail fixture");
    assert_eq!(item.planning_price_paise, Some(12_050));
    assert_eq!(item.price_basis, PriceBasis::CutOff);
    assert_eq!(item.lot_size, Some(10));
    assert_eq!(item.minimum_quantity, Some(10));
    assert_eq!(item.cost_per_lot_paise, Some(120_500));
    assert_eq!(item.minimum_application_amount_paise, Some(120_500));
    assert_eq!(item.allotment_date.as_deref(), Some("2026-09-03"));
    assert_eq!(item.listing_date.as_deref(), Some("2026-09-05"));
    assert_eq!(item.registrar_short_name.as_deref(), Some("KFintech"));
    assert_eq!(
        item.registrar_website.as_deref(),
        Some("https://kfintech.com")
    );
    assert_eq!(
        item.registrar_mapping_state,
        RegistrarMappingState::ConfirmedAlias
    );
    let dto = serde_json::to_string(&item).expect("safe DTO");
    assert!(!dto.contains("provider_issue_id"));
}

#[test]
fn minimum_quantity_rounds_up_to_complete_lots() {
    let item = parse_details_response(
        r#"{"status":"success","data":{"id":"ipo-1","name":"Example IPO","status":"open","issue_type":"regular","lot_size":10,"minimum_quantity":15,"cut_off_price":100}}"#,
        1,
    )
    .unwrap();

    assert_eq!(item.minimum_lots, Some(2));
    assert!(calculate_lot_plan(&item, 1, 1).is_err());
    assert_eq!(calculate_lot_plan(&item, 2, 1).unwrap().quantity, 20);
}

#[test]
fn missing_price_and_dates_are_tba_not_zero_or_invented() {
    let raw = r#"{"status":"success","data":[{"id":"future-ipo","symbol":"FUT","name":"Future IPO","status":"upcoming","issue_type":"regular","minimum_price":0,"maximum_price":0,"bidding_start_date":null,"bidding_end_date":null}]}"#;
    let item = &parse_list_response(raw, 1_756_633_600).expect("fixture")[0];
    assert_eq!(item.planning_price_paise, None);
    assert_eq!(item.price_basis, PriceBasis::Tba);
    assert_eq!(item.minimum_price_paise, None);
    assert_eq!(item.maximum_price_paise, None);
    assert_eq!(item.bidding_start_date, None);
    assert_eq!(item.bidding_end_date, None);
}

#[test]
fn rejects_invalid_id_and_malformed_envelope() {
    let raw = r#"{"status":"success","data":[{"id":"bad/id","symbol":"BAD","name":"Bad","status":"open","issue_type":"regular"}]}"#;
    assert!(parse_list_response(raw, 1_756_633_600).is_err());
    assert!(parse_list_response(r#"{"status":"error","data":[]}"#, 1_756_633_600).is_err());
}

#[test]
fn lot_plan_uses_selected_lots_accounts_and_integer_paise() {
    let item = parse_details_response(DETAIL_FIXTURE, 1_756_633_600).expect("detail fixture");
    let plan = sanket_desktop_lib::upstox::calculate_lot_plan(&item, 3, 2).expect("lot plan");

    assert_eq!(plan.quantity, 30);
    assert_eq!(plan.amount_per_account_paise, 361_500);
    assert_eq!(plan.total_capital_paise, 723_000);
    let snapshot = sanket_desktop_lib::upstox::metadata_snapshot(&item, 3, 2, None)
        .expect("metadata snapshot");
    assert_eq!(snapshot.metadata_source, "UPSTOX_IPO_API");
    assert_eq!(snapshot.source_ipo_id, "acme-ipo");
    let snapshot_json = serde_json::to_string(&snapshot).expect("safe metadata snapshot");
    assert!(!snapshot_json.contains("provider_issue_id"));
    assert_eq!(snapshot.lots, 3);
    assert_eq!(snapshot.quantity, 30);
    assert_eq!(snapshot.total_capital_paise, 723_000);
}

#[test]
fn metadata_revalidation_reports_price_lot_and_minimum_quantity_changes() {
    let original = parse_details_response(DETAIL_FIXTURE, 1_756_633_600).expect("detail fixture");
    let snapshot = sanket_desktop_lib::upstox::metadata_snapshot(&original, 1, 1, None)
        .expect("metadata snapshot");
    let mut changed = original.clone();
    changed.planning_price_paise = Some(13_000);
    changed.lot_size = Some(20);
    changed.minimum_quantity = Some(40);

    let changes = sanket_desktop_lib::upstox::metadata_changes(&snapshot, &changed);
    assert_eq!(changes.len(), 3);
    assert!(
        changes
            .iter()
            .any(|change| change.field == "planning price")
    );
    assert!(changes.iter().any(|change| change.field == "lot size"));
    assert!(
        changes
            .iter()
            .any(|change| change.field == "minimum quantity")
    );
}
