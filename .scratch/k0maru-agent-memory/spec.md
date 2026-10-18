# Spec 45: Isolated Cartridge Management (`k0maru pack install/uninstall/list` + DSH) & `distill --target omp`

## 1. Goal
1. **`distill --target omp` (`DistillTarget::Omp`)**:
   - Crystallize troubleshooting traces directly into `omp` native skills (`~/.omp/agent/skills/<slug>/SKILL.md` or `$OMP_HOME/skills/<slug>/SKILL.md`) with valid `name` + `description` frontmatter (`requireDescription: true` in `omp`).
2. **Environment-Level Isolated Pack & OpenHarness DSH Management (`k0maru pack install / uninstall / list`)**:
   - `k0maru pack install <source>`: accepts either a `.pack.json` file or an OpenHarness `DSH` directory (`harness.json` + `AGENTS.md` + `skills/`).
   - Installs markdown notes into `<vault>/.k0maru/packs/<pack_name>/` (indexed by `scan_markdown_files` into `cache.sqlite` and `GuardEngine` without polluting the user's primary vault folders) and mounts any `skills/<skill_name>/SKILL.md` into `<home>/.omp/agent/skills/<skill_name>/SKILL.md`.
   - `k0maru pack uninstall <pack_name>`: deletes `<vault>/.k0maru/packs/<pack_name>/`, removes mounted skill files/symlinks recorded in `pack-manifest.json`, and refreshes `cache.sqlite` for 100% zero-residue removal.
   - `k0maru pack list`: lists all isolated packs installed in `<vault>/.k0maru/packs/`.
