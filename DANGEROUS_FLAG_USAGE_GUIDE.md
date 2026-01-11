# Quick Guide: `--dangerously-skip-permissions`

## TL;DR

⚠️ **DANGER**: This flag bypasses ALL safety confirmations.

```bash
# Always commit first!
git add . && git commit -m "Before AI changes"

# Then use the flag
berrycode --dangerously-skip-permissions src/

# Roll back if needed
git reset --hard HEAD
```

## What It Does

**Skips confirmation for:**
- File writes/overwrites
- File deletions
- Git commits/pushes
- Code execution
- Database changes

## Basic Usage

```bash
# Preview changes first (recommended)
berrycode --dry-run --dangerously-skip-permissions src/

# Execute without prompts
berrycode --dangerously-skip-permissions src/

# With verbose logging
berrycode --dangerously-skip-permissions --verbose src/
```

## Safety Checklist

Before using this flag:

- [ ] ✅ Running in a Git repository
- [ ] ✅ All work is committed
- [ ] ✅ Tested with `--dry-run` first
- [ ] ✅ Have a backup/rollback plan
- [ ] ✅ Not on production system
- [ ] ✅ Trusting the AI model

## Error Messages

### Without flag (safe):
```
Error: Permission denied for dangerous command: /delete src/old.rs
Use --dangerously-skip-permissions to execute without confirmation
```

### With flag (dangerous):
```
⚠️  DANGEROUS MODE: Executing '/delete src/old.rs' without permission check
Successfully deleted: src/old.rs
```

## Comparison with `--yes-always`

| Flag | Safety Level | Use Case |
|------|-------------|----------|
| `--yes-always` | Medium risk | Skip routine confirmations |
| `--dangerously-skip-permissions` | **HIGH RISK** | Bypass ALL safety checks |

## When to Use

✅ **Good:**
- CI/CD pipelines (with Git protection)
- Automated testing
- Rapid prototyping (with frequent commits)

❌ **Never:**
- Production systems
- Without Git protection
- With uncommitted important changes
- With untrusted AI models

## Emergency Rollback

```bash
# Undo last commit
git reset --hard HEAD~1

# Restore specific file
git checkout HEAD -- path/to/file

# View what changed
git diff HEAD
```

## Implementation Details

See [DANGEROUSLY_SKIP_PERMISSIONS_IMPLEMENTATION.md](./DANGEROUSLY_SKIP_PERMISSIONS_IMPLEMENTATION.md) for:
- Technical architecture
- Code examples
- Test cases
- Security considerations
- Future enhancements

## Help

```bash
# Show help message
berrycode --help | grep -A 20 "dangerously-skip-permissions"

# View all flags
berrycode --help
```

## Related Flags

- `--dry-run`: Preview changes without executing
- `--yes-always`: Auto-approve routine prompts
- `--verbose`: Show detailed operation logs
- `--git` / `--no-git`: Enable/disable Git integration

## Examples

### Safe Workflow

```bash
# Step 1: Commit current state
git add .
git commit -m "Before AI refactoring"

# Step 2: Preview changes
berrycode --dry-run --dangerously-skip-permissions src/

# Step 3: Execute (if preview looks good)
berrycode --dangerously-skip-permissions src/

# Step 4: Review changes
git diff

# Step 5: Commit or rollback
git commit -am "AI refactoring" # or git reset --hard HEAD
```

### CI/CD Pipeline

```bash
#!/bin/bash
set -e

# Ensure clean working directory
git status --porcelain | grep -q '.' && exit 1

# Run AI with dangerous flag (safe in CI)
berrycode --dangerously-skip-permissions \
          --test-cmd "cargo test" \
          --auto-test \
          src/

# Verify tests still pass
cargo test

# Commit if successful
git commit -am "[CI] AI code improvements"
```

### Interactive Confirmation

```bash
# Preview first
berrycode --dry-run --dangerously-skip-permissions src/

# Ask user
echo "Proceed with execution? (y/n)"
read -r response

if [ "$response" = "y" ]; then
    berrycode --dangerously-skip-permissions src/
else
    echo "Aborted"
fi
```

---

**Remember**: With great power comes great responsibility. Always have a rollback plan!
