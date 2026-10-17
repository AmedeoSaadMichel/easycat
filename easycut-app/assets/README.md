# easycat visual assets

- `approved-mockup.png`: the original approved mockup, still used for the export decoration and portrait-preview background. UV coordinates are in `theme.rs`.
- `retina-brand.png`: dedicated 2172×724 artwork atlas derived from the approved mockup with the built-in ImageGen tool. The top strip supplies the panoramic header; the larger lower brand plate supplies the cat and wordmark. Both preserve their aspect ratio. The header is 88 logical points high and the brand plate is smaller than before, closer to the approved layout. Texture reduction uses bilinear interpolation when required by the renderer. The English tagline, controls and live values remain native text/widgets.
- `LilitaOne-Regular.ttf`: display font from [Google Fonts](https://github.com/google/fonts/tree/main/ofl/lilitaone), with SIL Open Font License in `LilitaOne-OFL.txt`. Missing glyphs fall back to the default egui fonts.
- `cosmic-header.png`, `Fredoka.ttf`, `Fredoka-OFL.txt`: retained reference assets from the earlier design; no longer embedded in the binary.
- Portrait backgrounds affect preview only: they do not crop, distort or enter exported video frames.
- All runtime assets are local; the app downloads nothing at startup.

## Earlier illustration prompt

The earlier header illustration was generated with the built-in ImageGen tool:


Generate a production bitmap asset for easycat desktop app HEADER BACKGROUND, very wide 3:1 composition. Psychedelic hand-inked cosmic alien landscape inspired by The Midnight Gospel cartoon aesthetic. Deep midnight aubergine #1d132a background, lilac mountains along bottom edge, surreal mushroom flora in coral pink and mint, ringed pale yellow planet in far RIGHT third, tiny handdrawn stars. LEFT TWO THIRDS almost entirely empty dark plum negative space to put live app logo and buttons on top; absolutely no lettering, no words, no UI, no labels, no frame, no watermark. Right third richly illustrated with a small pink three-eyed cat peeking above lavender mountains, third eye mystical, gentle mischievous expression. Flat 2D hand-drawn ink, delicate irregular outlines, dusty pastel acid colors, sophisticated cosmic illustration matching a warm dark purple video editor. Keep all elements low contrast enough to act as a background except pink cat and planet on far right. Wide cinematic landscape illustration, not a screenshot.


## Layout verification

`cargo test -p easycut-app easycat_layout_keeps_export_accessible` verifies Export/Cancel visibility at 1280×840, 1013×768 and 800×620 without opening a native window. It also checks the panel heading and portrait-only illustration background. Software previews are written to `target/ui-preview/` with generated demonstration content. A geometry test checks centering and aspect ratio. These previews do not replace interactive native-window testing.

## Retina artwork prompt

Edit the provided original easycat mockup into a dedicated high resolution artwork atlas for native desktop UI. Preserve the ORIGINAL exact illustration, lettering and pink cat identity, do not redesign. Output wide 3:1, ideally 3840x1280 or largest possible. Asset atlas, NOT an app screenshot. TOP QUARTER (y=0 to 25% of image): a continuous very thin panoramic HEADER landscape reproducing the reference top header, deep plum stars and coral lilac alien mountains across bottom, ringed planet and fungi on right. No text, no UI, no buttons in this top strip. Keep its art all inside that quarter, no crossing into lower zones. LOWER THREE QUARTERS: flat uniform deep plum #191123 background. In LOWER LEFT HALF put isolated original pink three-eyed cat face and original chunky buttery pale-yellow 'easycat' wordmark beside it, large crisp clean outlines; reproduce e a s y c a t exactly, no tagline, no additional letters, occupying x=3%..48%, y=40%..82%, comfortably padded. The rest of lower right is empty plum. Intended use: the UI will crop top 25% as wide header, and lower-left 50% from y=30% to95% as a separate brand plate. Faithfully retain whimsical inky Midnight Gospel inspired illustration details, sharp anti-aliased handdrawn contour lines, rich detail at Retina resolution. Absolutely no macOS traffic lights, window frame, Italian tagline, panels, timeline or video preview. This is a clean two-region production asset atlas, not a UI screenshot.

The returned atlas uses a taller top strip than requested; native UV coordinates follow the actual returned image. No 4K output is claimed.

## Fullscreen preview

Use the four-corner button at the right of the playback controls or press F. The app requests native fullscreen and displays only the preview and playback controls. Escape, F or Exit fullscreen returns to the editor. Portrait previews keep their illustrated side background. Selection and playhead are preserved. Tests verify keyboard actions, native viewport commands, layout, and exit even if a file load has cleared the current video. `target/ui-preview/easycat-fullscreen.png` uses a solid portrait test fixture, not a user video.

## Transparent decorations

`decorations-v2.png` is a dedicated RGBA atlas generated with ImageGen from the approved mockup as a style reference. The upper region contains purple/coral terrain; the lower region contains a complete teal botanical sprig and gold sparkle. Both have transparent backgrounds. The footer paints the terrain edge to edge behind status text. The estimate ornament preserves its aspect ratio and has its own reserved row space. Neither uses cropped UI screenshot pixels.

Prompt: Create one production UI decoration texture atlas on a true transparent background, inspired by the approved hand drawn cosmic purple/coral terrain and teal alien plants. No UI, text, boxes or panels. Landscape 1536×1024. Upper half: continuous shallow landscape border with low rocky hills and teal plants at the ends. Lower half: one complete botanical sprig with a gold sparkle, with padding around all tips. Preserve the reference linework and palette; transparent outside the painted ornaments.

## Panoramic footer and compact transport

`footer-panorama-v3.png` is a transparent panoramic cosmic terrain asset generated with ImageGen. Prompt: extremely shallow purple/lavender/coral rocky horizon with tiny teal alien plants, crisp inky detail, transparent outside the terrain, no text or UI, designed as a broad low footer rather than a tall scene. The returned 2172×724 atlas contains the terrain at y280–534. The renderer repeats that region horizontally at its original aspect ratio, clipped only at the window edge; resizing never distorts its proportions.

Playback and volume share one row. Playback stays centered, volume remains on the left, and fullscreen remains on the right. The editor has no scroll area: preview and thumbnail heights adapt to leave Start and End visible, including at 720×480. The estimate ornament has a larger reserved area.

The export panel also avoids page scrolling. Below 820 points of height, an operation selector replaces the three large cards and descriptive labels move out of the compact layout. Track compatibility details open in a menu. Estimate actions are available from the estimate label's context menu, and from Estimate options in the spacious layout. Compact footer text and spacing keep settings separate from the estimate at the minimum size. Layout tests cover the default 900×640 window and 720×480 compression, target-size and trim modes.

## Themed window chrome

macOS keeps its native traffic-light controls over a transparent titlebar and full-size illustrated content view, with the title text hidden. The logo leaves the native buttons clear. Windows and Linux use an undecorated window with theme-colored minimize, maximize/restore and close buttons in the illustrated header. Empty header space drags the window; double-click toggles maximization. The body remains opaque. Border resizing uses native BeginResize on Windows/Linux and the existing client-edge fallback on macOS. Tests verify viewport configuration, window-control commands and header dragging. Native visual behavior on Windows/Linux still requires testing on those systems.

Estimate options now open by clicking the estimate heading (Rough estimate, Sample-based estimate or Requested size). The popup belongs to the estimate section and is disabled during export. The settings panel no longer contains a separate Estimate options row. A layout regression switches from Change container to Compress and checks that settings remain above the estimate.

The current native starting and minimum window size is 1180×850 logical points, based on the approved desktop screenshot. Smaller headless fixtures remain as layout stress tests. The export section disables the panel's automatic separator and retains one explicit rule above the estimate.
