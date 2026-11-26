# Miro Test44: Dismiss Individual Recent Tasks

Includes the UI-only navigation-inset and recent-task clipping repairs
documented in MIRO-TEST43.md, plus per-task close controls.

Each card header has a close icon. It removes only that task's recent-card
entry and parked navigation stack. Other cards keep their order and can
still be resumed. Targets use the task kind rather than the current grid
index, so an old duplicate close cannot remove its neighbor after reflow.
Closing the final card clears the stored backdrop and returns to Skyward
home. Scrolling is clamped to the shorter grid after dismissal.

This is not process termination. Active installation, backup, Root work,
downloads and the browser engine are not cancelled, and their state is not
cleared. Reopening a feature can create a recent card again. The existing
bottom clear-all control remains available.

Tests exercise real taps on the close target, overlapping card-hit priority,
middle/last-card dismissal, stale duplicate actions, remaining-task resume,
scroll clamping, re-opening, and preservation of background-operation state.
The shared build and preview workflows cover two resolutions, light/dark
themes and three languages. Phone gesture acceptance remains pending.

ROM installation, extraction, AVB, slot selection, snapshots, recovery
retention, device configuration, decryption, touch and browser runtime
remain the test40 baseline; KernelSU and module installation stay as test41.
No phone flash or Root installation is performed by the offline workflow.
Browser-enabled images remain private under
crates/skyward-browser/NOTICE.md.
