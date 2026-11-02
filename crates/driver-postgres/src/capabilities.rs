use driver_core::{
    capabilities::{BulkSupport, Capabilities, ExplainSupport},
    connection::ServerVersion,
};

/// Capabilities known before connecting.
/// Assumes a modern server; `for_version` turns off what older ones lack.
pub fn static_capabilities() -> Capabilities {
    Capabilities {
        // Structure
        schemas: true,
        multiple_databases: false, // switching database needs a new connection
        views: true,
        materialized_views: true,
        sequences: true,
        functions: true,
        procedures: true,
        triggers: true,
        custom_types: true,
        extensions: true,
        partitions: true,
        check_constraints: true,
        foreign_keys: true,
        comments: true,

        // Execution
        transactions: true,
        savepoints: true,
        transactional_ddl: true,
        error_aborts_transaction: true,
        cancel_query: true,
        multiple_result_sets: false, // TODO: execute returns a single ExecOutcome (see TODO.md, driver-core)
        server_notices: false,       // TODO: no notice API in driver-core yet
        listen_notify: false,        // TODO: no LISTEN/NOTIFY API in driver-core yet
        parameters: false,           // TODO: no execute_params on Connection yet
        streaming_cursor: false, // TODO: rows are streamed without cursor paging (see stream.rs)

        // Analysis
        explain: ExplainSupport::JsonWithAnalyze,
        query_timeout: true, // statement_timeout
        row_estimate: true,

        // Data
        editable_results: false, // TODO: ColumnMeta::source_table not filled in yet
        returning_clause: true,
        merge_statement: true,
        bulk_import: BulkSupport::None, // TODO: no bulk API in driver-core yet (PG: COPY)
        bulk_export: BulkSupport::None, // TODO: no bulk API in driver-core yet (PG: COPY)

        // Administration
        roles_and_grants: false, // TODO: no admin API in driver-core yet
        active_sessions: false,  // TODO: no admin API in driver-core yet (pg_stat_activity)
        kill_session: false,     // TODO: no admin API in driver-core yet (pg_terminate_backend)
        table_stats: false,      // TODO: introspection doesn't return sizes / row counts yet

        // Connection
        tls: true,
        ssh_tunnel: false, // TODO: tunnel not implemented in this driver yet
        connection_string: true,
    }
}

/// Narrow the static set by server version (procedures 11+, MERGE 15+, ...).
pub fn for_version(version: &ServerVersion) -> Capabilities {
    let mut caps = static_capabilities();

    caps.explain = if version.at_least(9, 0) {
        ExplainSupport::JsonWithAnalyze
    } else {
        ExplainSupport::Text
    };
    caps.extensions = version.at_least(9, 1);
    caps.materialized_views = version.at_least(9, 3);
    caps.partitions = version.at_least(10, 0);
    caps.procedures = version.at_least(11, 0);
    caps.merge_statement = version.at_least(15, 0);

    caps
}
