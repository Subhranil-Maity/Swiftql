use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    // Structure
    pub schemas: bool,            // PG: yes; MySQL: database = schema
    pub multiple_databases: bool, // switch database without reconnecting
    pub views: bool,
    pub materialized_views: bool,
    pub sequences: bool,
    pub functions: bool,
    pub procedures: bool, // PG 11+
    pub triggers: bool,
    pub custom_types: bool, // enums, composite types, domains
    pub extensions: bool,
    pub partitions: bool,        // PG 10+ declarative partitioning
    pub check_constraints: bool, // MySQL 8.0.16+
    pub foreign_keys: bool,
    pub comments: bool, // COMMENT ON table/column

    // Execution
    pub transactions: bool,
    pub savepoints: bool,
    pub transactional_ddl: bool, // PG: yes; MySQL: no (DDL auto-commits)
    pub error_aborts_transaction: bool, // PG: yes; MySQL: no
    pub cancel_query: bool,
    pub multiple_result_sets: bool,
    pub server_notices: bool, // RAISE NOTICE
    pub listen_notify: bool,
    pub parameters: bool,       // execute_params supported
    pub streaming_cursor: bool, // portal/cursor paging for large results

    // Analysis
    pub explain: ExplainSupport,
    pub query_timeout: bool,
    pub row_estimate: bool, // EXPLAIN-based row estimates

    // Data
    pub editable_results: bool, // needs source TableRef + primary key
    pub returning_clause: bool, // INSERT ... RETURNING
    pub merge_statement: bool,  // PG 15+
    pub bulk_import: BulkSupport,
    pub bulk_export: BulkSupport,

    // Administration
    pub roles_and_grants: bool,
    pub active_sessions: bool, // pg_stat_activity / SHOW PROCESSLIST
    pub kill_session: bool,
    pub table_stats: bool, // sizes, row estimates

    // Connection
    pub tls: bool,
    pub ssh_tunnel: bool,
    pub connection_string: bool, // parse postgres://... URLs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ExplainSupport {
    None,
    Text,
    Json,
    JsonWithAnalyze, // EXPLAIN (ANALYZE, FORMAT JSON)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum BulkSupport {
    None,
    Insert,       // fallback: batched INSERTs
    CopyProtocol, // PG COPY
    LoadData,     // MySQL LOAD DATA
}
