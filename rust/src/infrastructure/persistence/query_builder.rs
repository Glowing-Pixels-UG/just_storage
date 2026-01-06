/// Common SQL query fragments to reduce duplication and ensure consistency
pub struct QueryBuilder;

impl QueryBuilder {
    /// Base SELECT clause for object queries
    pub const OBJECT_SELECT: &'static str = r#"
        SELECT id, namespace, tenant_id, key, status, storage_class,
               content_hash, size_bytes, content_type, metadata,
               created_at, updated_at
        FROM objects
    "#;

    /// WHERE clause for committed objects only
    pub const COMMITTED_WHERE: &'static str = "WHERE status = 'COMMITTED'";

    /// Build WHERE clause with namespace and tenant filter
    pub fn namespace_tenant_where(_namespace: &str, _tenant_id: &str) -> String {
        format!(
            "{} AND namespace = $1 AND tenant_id = $2",
            Self::COMMITTED_WHERE
        )
    }

    /// Build WHERE clause with namespace, tenant, and key filter
    #[allow(dead_code)]
    pub fn namespace_tenant_key_where(_namespace: &str, _tenant_id: &str, _key: &str) -> String {
        format!(
            "{} AND namespace = $1 AND tenant_id = $2 AND key = $3",
            Self::COMMITTED_WHERE
        )
    }
}
