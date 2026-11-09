use driver_core::{
    capabilities::{BulkSupport, Capabilities, ExplainSupport},
    connection::ServerVersion,
};

/// Capabilities known before connecting. SQLite is compiled in, so the
/// version is known too; `for_version` only matters for an older library.
pub fn static_capabilities() -> Capabilities {
    Capabilities {
        // Structure
        schemas: true, // main, temp and ATTACHed databases
        multiple_databases: false,
        views: true,
        materialized_views: false,
        sequences: false,
        functions: false, // only C-level application functions
        procedures: false,
        triggers: true,
        custom_types: false,
        extensions: false, // TODO: loadable extensions (load_extension) aren't offered
        partitions: false,
        check_constraints: true,
        foreign_keys: true,
        comments: false,

        // Execution
        transactions: true,
        savepoints: true,
        transactional_ddl: true,
        error_aborts_transaction: false, // a failed statement leaves the transaction open
        cancel_query: true,              // sqlite3_interrupt
        multiple_result_sets: false,
        server_notices: false,
        listen_notify: false,
        parameters: false, // TODO: no execute_params on Connection yet
        streaming_cursor: false,

        // Analysis
        explain: ExplainSupport::Text, // EXPLAIN QUERY PLAN
        query_timeout: true,           // progress handler + interrupt
        row_estimate: false,

        // Data
        editable_results: false, // TODO: ColumnMeta::source_table not filled in yet
        returning_clause: true,  // 3.35+
        merge_statement: false,
        bulk_import: BulkSupport::None,
        bulk_export: BulkSupport::None,

        // Administration
        roles_and_grants: false,
        active_sessions: false,
        kill_session: false,
        table_stats: false, // TODO: sizes need the dbstat virtual table

        // Connection
        tls: false,
        ssh_tunnel: false,
        connection_string: false,
    }
}

pub fn for_version(version: &ServerVersion) -> Capabilities {
    let mut caps = static_capabilities();
    caps.returning_clause = version.at_least(3, 35);
    caps
}
