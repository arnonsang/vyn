# vyn add / del

## vyn add

Explicitly add files or directories to vault tracking.

```bash
vyn add <paths...>
```

Adds the specified paths to the manifest so they will be included in the next `vyn push`.

**Example:**

```bash
vyn add .env .env.staging secrets/
```

---

## vyn del

Remove files or directories from vault tracking.

```bash
vyn del <paths...>
```

Removes the specified paths from the manifest. The files remain on disk but will no longer be pushed or pulled.

**Example:**

```bash
vyn del .env.old
```

## Notes

- Both commands update `.vyn/manifest.json` immediately; the change takes effect on the next `vyn push`
- Tracked files are also subject to `.vynignore` rules; adding a path that matches an ignore rule has no effect
