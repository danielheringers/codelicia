# VBS Parity Artifacts

This directory tracks parity between `vibing-steampunk` (VBS) and the Neuro runtime/tooling.

## Files

- `vbs-contract/tools.json`: canonical VBS tool catalog extracted from `GetAllToolNames()`.
- `vbs-contract/endpoints.json`: ADT endpoint paths discovered in VBS production code.
- `gap-list.md`: parity gap report (catalog vs functionally implemented).

## Regenerate

```powershell
powershell -ExecutionPolicy Bypass -File scripts/neuro-parity/generate_vbs_parity.ps1
```

The script assumes local paths:

- VBS: `C:/Users/danie/OneDrive/Documentos/Projetos/vibing-steampunk`
- Neuro: current `codex-rs` workspace

You can override with parameters:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/neuro-parity/generate_vbs_parity.ps1 `
  -VbsRoot "D:/path/to/vibing-steampunk" `
  -NeuroRoot "D:/path/to/codex-rs"
```
