# vyn st / diff

## vyn st

Show changes in the working directory against the last push baseline.

```bash
vyn st [-v]
```

| Flag | Description |
|---|---|
| `-v` / `--verbose` | Include inline unified diffs for text files and size summaries for binary files |

**Output example:**

```
Modified  .env
Added     .env.staging
Deleted   .env.old
```

---

## vyn diff

Show a unified diff against the baseline manifest.

```bash
vyn diff [file]
```

| Argument | Description |
|---|---|
| `file` | Optional. Diff only this path. If omitted, diffs all changed paths. |

**Output example:**

```diff
--- .env  (baseline)
+++ .env  (local)
@@ -1,4 +1,5 @@
 DATABASE_URL=postgres://localhost/mydb
-API_KEY=old_key
+API_KEY=new_key
+CACHE_TTL=300
```

**Notes:**
- Binary files are shown as a size-change summary: `Binary file .env.db changed (512 -> 1024 bytes)`
- Returns an error if the specified file is not tracked in the baseline manifest
- The diff compares against `.vyn/manifest.json` — the state at the last `vyn push` or `vyn pull`
