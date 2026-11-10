# Read-only review of haotian AOD

Reviewed Rinrroy/skyward-for-haotian main 0cfe1a08609d7c7233535f2861b8eff6aff105c6.
No changes to miro hardware configuration or AOD behavior are made here.

AOD existed before our d6e7b7a baseline; the four newer commits mostly change
how double-tap wake shares that doze mode and expose its default-off preference.

- skyward-ui/src/aod.rs draws a black OLED frame with date/time, battery, slot
  and data-lock state. The face shifts horizontally among five positions each
  minute (-16, -8, 0, 8, 16 dp) to reduce sustained illumination of one spot.
- devices/haotian.toml sets feature ID 3 to 1 (doze_brightness high), sends
  command set 44 (mi,mdss-dsi-doze-hbm-command), and exits using set 18 (nolp).
  These are panel-specific numbers, not generic commands for other phones.
- skyward/src/aod.rs reads feature names from mi_disp's disp_param and sends
  configured command-set IDs through the panel command node. Requests contain
  a newline. EAGAIN retries six times at 12 ms; three consecutive command-set
  refusals disable this doze path for the run, with nonfatal fallback.
- With AOD on the loop paints the face then leaves scanout/panel in doze, rather
  than normal DRM power-off. With double-tap on but AOD off it paints black and
  retains doze, keeping the touch controller scanning. This prevents suspend
  in the measured haotian fallback. AOD/default-off switches do not magically
  provide gesture IRQ wake from full display power-off or CPU suspend.
- Miro retains keep_panel_on=true and its existing black-frame compatibility.
  Neither haotian command sets nor doze brightness controls are enabled on miro.
