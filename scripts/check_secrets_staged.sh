#!/usr/bin/env bash
set -euo pipefail

# Secret scan for staged files only.
# Usage:
#   scripts/check_secrets_staged.sh
#
# Skip once:
#   SKIP_SECRET_SCAN=1 git commit -m "..."

if [[ "${SKIP_SECRET_SCAN:-0}" == "1" ]]; then
  echo "[secret-scan] SKIPPED (SKIP_SECRET_SCAN=1)"
  exit 0
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "[secret-scan] Not a git repository."
  exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "[secret-scan] ripgrep (rg) is required."
  exit 1
fi

# Sensitive patterns: generic tokens + common provider key formats.
PATTERNS='(OPENAI_API_KEY|OPENROUTER_API_KEY|GEMINI_API_KEY|ANTHROPIC_API_KEY|xox[baprs]-[0-9A-Za-z-]{10,}|sk-[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{20,}|ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|-----BEGIN (RSA|EC|OPENSSH|PRIVATE) KEY-----|api[_-]?key\s*[:=]\s*["'\'']?[A-Za-z0-9._-]{16,}|\btoken\b\s*[:=]\s*["'\'']?[A-Za-z0-9._-]*[0-9][A-Za-z0-9._-]{15,})'

# Only staged files that are added/copied/modified/renamed.
STAGED_FILES="$(git diff --cached --name-only --diff-filter=ACMR)"

if [[ -z "${STAGED_FILES}" ]]; then
  echo "[secret-scan] No staged files."
  exit 0
fi

FOUND=0
while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  [[ -f "$file" ]] || continue

  # Skip known non-source artifacts and docs snapshots that may contain synthetic tokens.
  case "$file" in
    scripts/check_secrets_staged.sh|.githooks/pre-commit|*.png|*.jpg|*.jpeg|*.gif|*.pdf|*.zip|*.lock|package-lock.json|pnpm-lock.yaml)
      continue
      ;;
  esac

  # Scan the staged version (not working tree).
  if git show ":$file" | rg -n -i --pcre2 "$PATTERNS" >/tmp/secret_scan_hit.txt 2>/dev/null; then
    echo ""
    echo "[secret-scan] Possible secret in staged file: $file"
    sed -n '1,5p' /tmp/secret_scan_hit.txt
    FOUND=1
  fi
done <<EOF
$STAGED_FILES
EOF

rm -f /tmp/secret_scan_hit.txt

if [[ $FOUND -eq 1 ]]; then
  echo ""
  echo "[secret-scan] Commit blocked."
  echo "If this is intentional, rotate/redact first, or bypass once:"
  echo "  SKIP_SECRET_SCAN=1 git commit -m \"...\""
  exit 1
fi

echo "[secret-scan] OK"
exit 0
