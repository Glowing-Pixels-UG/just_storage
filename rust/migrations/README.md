# Migrations

SQLx migrations, applied in filename order on startup.

## Convention

- **4-digit, zero-padded, monotonic** prefix: `0001_`, `0002_`, … `0010_`.
- One logical change per migration; name describes intent
  (`0009_add_sessions_table.sql`).
- **Never renumber or edit an already-applied migration** — migrations are
  append-only. To change a deployed schema, add a new migration.

## Note on history

Early files used inconsistent prefixes (`003_`, `004_`, `005_` alongside
`0001_`, `0006_`–`0009_`). These are already applied in deployed databases, so
they are intentionally left as-is. New migrations must follow the 4-digit
convention above. The `0006`–`0008` `fix_gc_*` series corrected the GC stored
functions and is settled.
