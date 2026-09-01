use sanket_desktop_lib::upstox::{
    IpoStatus, PriceBasis, RegistrarMappingState, parse_details_response, parse_list_response,
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
