const IPO_ISSUES = [
  { clientId: "90000000001", name: "SYNTHETIC ALPHA LIMITED" },
  { clientId: "90000000002", name: "SYNTHETIC BETA LIMITED" },
];
const LOOKUP_CONTRACT = {
  endpoint: "https://0uz601ms56.execute-api.ap-south-1.amazonaws.com/prod/api/query",
  types: ["pan", "appno", "dpclid"],
  headers: ["client_id", "reqparam"],
};
