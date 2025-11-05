# Feeder Vision — Product Spec (v0.1)
## Problem
Users point to an feeder camera SD card dump folder with thousands of frames; want animal presence + species offline and potentially file and folder reorganization.

## Scope v0
- Stage A: optional “animal present” filter (YOLO-n or heuristic motion).
- Stage B3: CLIP embeddings + HNSW k-NN over local gallery.
- Open-set: abstain if cos(top1) < T_min or (top1-top2) < Δ_min.

## Deliverables
- GUI (egui): folder ingest, grid, review-uncertain tray, “Add to reference”, start with UI in Dutch only, prepare for multi-language support.
- CSV export: file,present,species,confidence.
- File reorganization: retain only files with animal presence, sort into species folders.
- Reference pack updater (check for updates, manual import).

## Non-goals v0
- Training, cloud inference, mobile, multi-user sync.

## Performance targets
- 5k frames < 10 min on i5/16GB (no GPU), skipping 80% as empty.
