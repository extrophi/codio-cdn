# CODIO CDN SETUP CHECKLIST
## Phase 1 Foundation - GitHub Setup

**Based on successful IAC-033 Extrophi Ecosystem pattern**

---

## OVERVIEW

**Goal:** Set up GitHub infrastructure for 4 parallel agents  
**Pattern:** Same as IAC-033 (proven to work)  
**Duration:** 10 minutes setup, then spawn agents  

---

## CHECKLIST

### ☐ 1. Create GitHub Repository

```bash
gh repo create extrophi/codio-cdn \
  --public \
  --description "Decentralized CDN - Phase 1 Foundation"
```

**Verify:** Visit https://github.com/extrophi/codio-cdn

---

### ☐ 2. Initialize Git

```bash
cd /Users/kjd/01-projects/IAC-034-codio-cdn
git init
git add .
git commit -m "docs: Initial commit"
git branch -M main
git remote add origin https://github.com/extrophi/codio-cdn.git
git push -u origin main
```

**Verify:** `git log` shows commit, `git remote -v` shows origin

---

### ☐ 3. Create Agent Branches

```bash
git branch alpha
git branch beta  
git branch gamma
git branch delta

git push origin alpha beta gamma delta
```

**Verify:** `gh repo view --web` shows 5 branches (main + 4 agents)

---

### ☐ 4. Create Labels

```bash
gh label create "agent:alpha" --color "0075ca"
gh label create "agent:beta" --color "1d76db"
gh label create "agent:gamma" --color "0e8a16"
gh label create "agent:delta" --color "fbca04"
gh label create "phase:1" --color "d93f0b"
gh label create "status:pending" --color "ededed"
```

**Verify:** `gh label list` shows 6+ labels

---

### ☐ 5. Create Prompt Files

Create `.github/prompts/` with 4 agent prompts:
- `alpha-content-id.md`
- `beta-dht.md`
- `gamma-cli.md`
- `delta-tests.md`

**Verify:** `ls .github/prompts/` shows 4 files

---

### ☐ 6. Create GitHub Actions

Create `.github/workflows/rust-ci.yml`

**Verify:** `ls .github/workflows/` shows workflow file

---

### ☐ 7. Create GitHub Issues

```bash
gh issue create --title "[ALPHA] ..." --body "$(cat .github/prompts/alpha-content-id.md)" --label "agent:alpha,phase:1"
gh issue create --title "[BETA] ..." --body "$(cat .github/prompts/beta-dht.md)" --label "agent:beta,phase:1"
gh issue create --title "[GAMMA] ..." --body "$(cat .github/prompts/gamma-cli.md)" --label "agent:gamma,phase:1"
gh issue create --title "[DELTA] ..." --body "$(cat .github/prompts/delta-tests.md)" --label "agent:delta,phase:1"
```

**Verify:** `gh issue list` shows 4 open issues

---

### ☐ 8. Push GitHub Files

```bash
git add .github/
git commit -m "ci: Add workflows and prompts"
git push origin main
```

**Verify:** GitHub repo shows `.github/` directory

---

### ☐ 9. Verification Commands

```bash
# Check repo exists
gh repo view

# Check branches
git branch -a

# Check labels
gh label list

# Check issues
gh issue list --label "phase:1"

# Check workflows
ls .github/workflows/
```

**Expected Output:**
- 5 branches (main, alpha, beta, gamma, delta)
- 6+ labels
- 4 open issues
- 1 workflow file

---

## SUCCESS CRITERIA

✅ All checklist items complete  
✅ 4 issues visible on GitHub  
✅ 4 branches ready for agents  
✅ CI/CD workflow in place  
✅ Prompt files committed  

---

## NEXT STEP

**Spawn 4 CCW agents:**

Each agent:
1. Reads their GitHub issue
2. Reads their prompt file
3. Checks out their branch
4. Implements their task
5. Pushes to their branch
6. Updates their issue

---

## COMMON ISSUES

**Issue:** `gh: command not found`  
**Fix:** `brew install gh` then `gh auth login`

**Issue:** Push rejected (no workflow scope)  
**Fix:** `gh auth refresh -s workflow`

**Issue:** Can't create repo (already exists)  
**Fix:** `gh repo delete extrophi/codio-cdn` then retry

---

## TIMELINE

**Setup:** 10 minutes (this checklist)  
**Agent work:** 4-5 hours (parallel)  
**Total:** ~5 hours to Phase 1 complete  

---

**BASED ON:** IAC-033 successful setup (Wave 1 completed)  
**CONFIDENCE:** HIGH (proven pattern)  
**STATUS:** Ready to execute  
